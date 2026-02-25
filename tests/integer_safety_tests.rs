//! Integration tests for integer safety and memory bounds validation.
//!
//! These tests use a mock TCP server to simulate malicious or faulty ZK devices
//! that send negative integers, overflow values, or inconsistent sizes.

use byteorder::{ByteOrder, LittleEndian, WriteBytesExt};
use rustzk::constants::*;
use rustzk::protocol::{TCPWrapper, ZKPacket};
use rustzk::{ZKProtocol, ZK};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

/// Helper: reads one TCP-wrapped ZK packet from the stream
fn read_one_packet(stream: &mut std::net::TcpStream) -> ZKPacket<'static> {
    let mut header = [0u8; 8];
    stream.read_exact(&mut header).unwrap();
    let (length, _) = TCPWrapper::decode_header(&header).unwrap();
    let mut body = vec![0u8; length];
    stream.read_exact(&mut body).unwrap();
    ZKPacket::from_bytes_owned(body).unwrap()
}

/// Helper: sends a ZK packet wrapped in TCP framing
fn send_packet(stream: &mut std::net::TcpStream, packet: &ZKPacket) {
    stream
        .write_all(&TCPWrapper::wrap(&packet.to_bytes()))
        .unwrap();
    stream.flush().unwrap();
}

/// Helper: handle CMD_CONNECT and return session_id
fn handle_connect(stream: &mut std::net::TcpStream, session_id: u16) {
    let packet = read_one_packet(stream);
    assert_eq!(packet.command, CMD_CONNECT);
    let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, vec![]);
    send_packet(stream, &res);
}

/// Helper: handle any packet with ACK_OK
fn handle_ack(stream: &mut std::net::TcpStream, session_id: u16) {
    let packet = read_one_packet(stream);
    let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, vec![]);
    send_packet(stream, &res);
}

/// Helper: handle CMD_GET_FREE_SIZES with custom field values
fn handle_get_free_sizes(
    stream: &mut std::net::TcpStream,
    session_id: u16,
    users: i32,
    fingers: i32,
    records: i32,
) {
    let packet = read_one_packet(stream);
    assert_eq!(packet.command, CMD_GET_FREE_SIZES);
    let mut bytes = vec![0u8; 92];
    // fields[4] = users at offset 16
    <LittleEndian as ByteOrder>::write_i32(&mut bytes[16..20], users);
    // fields[6] = fingers at offset 24
    <LittleEndian as ByteOrder>::write_i32(&mut bytes[24..28], fingers);
    // fields[8] = records at offset 32
    <LittleEndian as ByteOrder>::write_i32(&mut bytes[32..36], records);
    // faces at offset 80 (first 4 bytes of the 80..92 range)
    <LittleEndian as ByteOrder>::write_i32(&mut bytes[80..84], -5);
    let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, bytes);
    send_packet(stream, &res);
}

/// Helper: handle TZAdj sync
fn handle_tzadj(stream: &mut std::net::TcpStream, session_id: u16) {
    let packet = read_one_packet(stream);
    assert_eq!(packet.command, CMD_OPTIONS_RRQ);
    let res = ZKPacket::new(
        CMD_ACK_OK,
        session_id,
        packet.reply_id,
        b"TZAdj=7\0".to_vec(),
    );
    send_packet(stream, &res);
}

// =============================================================================
// Test 1: Device sends negative i32 as template total_size
// =============================================================================

#[test]
fn test_negative_template_total_size() {
    let _ = env_logger::builder().is_test(true).try_init();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let sid = 1234;

        handle_connect(&mut stream, sid);
        // read_sizes (get_templates calls read_sizes first)
        handle_get_free_sizes(&mut stream, sid, 0, 1, 0);
        handle_tzadj(&mut stream, sid);

        // _CMD_PREPARE_BUFFER for template read
        let packet = read_one_packet(&mut stream);
        assert_eq!(packet.command, _CMD_PREPARE_BUFFER);
        let mut res_payload = vec![0u8; 5];
        res_payload[0] = 1;
        // Total size including 4-byte header: size = 8 (4 header + 4 template data)
        <LittleEndian as ByteOrder>::write_u32(&mut res_payload[1..5], 8);
        let res = ZKPacket::new(CMD_PREPARE_DATA, sid, packet.reply_id, res_payload);
        send_packet(&mut stream, &res);

        // _CMD_READ_BUFFER
        let packet = read_one_packet(&mut stream);
        assert_eq!(packet.command, _CMD_READ_BUFFER);

        // Send CMD_DATA with a NEGATIVE i32 as total_size
        let mut data = Vec::new();
        data.write_i32::<LittleEndian>(-1).unwrap(); // Negative total_size!
        data.extend_from_slice(&[0xAA; 4]); // Some junk data
        let res = ZKPacket::new(CMD_DATA, sid, packet.reply_id, data);
        send_packet(&mut stream, &res);

        // Handle CMD_FREE_DATA
        handle_ack(&mut stream, sid);
    });

    let mut zk = ZK::new("127.0.0.1", port);
    zk.connect(ZKProtocol::TCP).unwrap();

    let result = zk.get_templates();
    assert!(
        result.is_err(),
        "Should reject negative template total_size"
    );
    let err = format!("{}", result.unwrap_err());
    assert!(
        err.contains("Negative template data size"),
        "Error should mention negative size, got: {}",
        err
    );

    server.join().unwrap();
}

// =============================================================================
// Test 2: Device sends negative users/fingers/records in read_sizes
// =============================================================================

#[test]
fn test_negative_device_fields_clamped_to_zero() {
    let _ = env_logger::builder().is_test(true).try_init();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let sid = 5678;

        handle_connect(&mut stream, sid);
        // Send negative values for users, fingers, records
        handle_get_free_sizes(&mut stream, sid, -10, -20, -30);
        handle_tzadj(&mut stream, sid);
    });

    let mut zk = ZK::new("127.0.0.1", port);
    zk.connect(ZKProtocol::TCP).unwrap();

    // read_sizes should succeed — clamping negatives to 0
    zk.read_sizes().unwrap();
    assert_eq!(zk.users(), 0, "Negative users should be clamped to 0");
    assert_eq!(zk.fingers(), 0, "Negative fingers should be clamped to 0");
    assert_eq!(zk.records(), 0, "Negative records should be clamped to 0");
    assert_eq!(zk.faces(), 0, "Negative faces should be clamped to 0");

    server.join().unwrap();
}

// =============================================================================
// Test 3: Device claims users > 0 but total_size = 0 in user data
// =============================================================================

#[test]
fn test_zero_total_size_with_nonzero_users() {
    let _ = env_logger::builder().is_test(true).try_init();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let sid = 9999;

        handle_connect(&mut stream, sid);
        // Claim 5 users exist
        handle_get_free_sizes(&mut stream, sid, 5, 0, 0);
        handle_tzadj(&mut stream, sid);

        // _CMD_PREPARE_BUFFER
        let packet = read_one_packet(&mut stream);
        assert_eq!(packet.command, _CMD_PREPARE_BUFFER);
        let mut res_payload = vec![0u8; 5];
        res_payload[0] = 1;
        // 4 bytes for total_size header only
        <LittleEndian as ByteOrder>::write_u32(&mut res_payload[1..5], 4);
        let res = ZKPacket::new(CMD_PREPARE_DATA, sid, packet.reply_id, res_payload);
        send_packet(&mut stream, &res);

        // _CMD_READ_BUFFER
        let packet = read_one_packet(&mut stream);
        assert_eq!(packet.command, _CMD_READ_BUFFER);

        // Send CMD_DATA with total_size = 0 (but device claimed 5 users)
        let mut data = Vec::new();
        data.write_u32::<LittleEndian>(0).unwrap(); // total_size = 0
        let res = ZKPacket::new(CMD_DATA, sid, packet.reply_id, data);
        send_packet(&mut stream, &res);

        // CMD_FREE_DATA
        handle_ack(&mut stream, sid);
    });

    let mut zk = ZK::new("127.0.0.1", port);
    zk.connect(ZKProtocol::TCP).unwrap();

    // Should return empty vec safely instead of panicking on division by zero
    let users = zk.get_users().unwrap();
    assert_eq!(users.len(), 0, "Zero total_size should yield 0 users");

    server.join().unwrap();
}

// =============================================================================
// Test 4: Device sends oversized total_size (0xFFFFFFFF) for user data
// =============================================================================

#[test]
fn test_oversized_total_size_rejected() {
    let _ = env_logger::builder().is_test(true).try_init();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let sid = 4321;

        handle_connect(&mut stream, sid);
        // Claim 1 user
        handle_get_free_sizes(&mut stream, sid, 1, 0, 0);
        handle_tzadj(&mut stream, sid);

        // _CMD_PREPARE_BUFFER
        let packet = read_one_packet(&mut stream);
        assert_eq!(packet.command, _CMD_PREPARE_BUFFER);
        let mut res_payload = vec![0u8; 5];
        res_payload[0] = 1;
        <LittleEndian as ByteOrder>::write_u32(&mut res_payload[1..5], 8);
        let res = ZKPacket::new(CMD_PREPARE_DATA, sid, packet.reply_id, res_payload);
        send_packet(&mut stream, &res);

        // _CMD_READ_BUFFER
        let packet = read_one_packet(&mut stream);
        assert_eq!(packet.command, _CMD_READ_BUFFER);

        // Send CMD_DATA with total_size = 0xFFFFFFFF (4GB!)
        let mut data = Vec::new();
        data.write_u32::<LittleEndian>(0xFFFFFFFF).unwrap(); // Absurdly large
        data.extend_from_slice(&[0x00; 4]); // Some padding
        let res = ZKPacket::new(CMD_DATA, sid, packet.reply_id, data);
        send_packet(&mut stream, &res);

        // CMD_FREE_DATA
        handle_ack(&mut stream, sid);
    });

    let mut zk = ZK::new("127.0.0.1", port);
    zk.connect(ZKProtocol::TCP).unwrap();

    let result = zk.get_users();
    assert!(result.is_err(), "Should reject oversized total_size");
    let err = format!("{}", result.unwrap_err());
    assert!(
        err.contains("exceeds maximum"),
        "Error should mention exceeds maximum, got: {}",
        err
    );

    server.join().unwrap();
}

// =============================================================================
// Test 5: Oversized template total_size is rejected
// =============================================================================

#[test]
fn test_oversized_template_total_size_rejected() {
    let _ = env_logger::builder().is_test(true).try_init();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let sid = 7777;

        handle_connect(&mut stream, sid);
        handle_get_free_sizes(&mut stream, sid, 0, 1, 0);
        handle_tzadj(&mut stream, sid);

        // _CMD_PREPARE_BUFFER
        let packet = read_one_packet(&mut stream);
        assert_eq!(packet.command, _CMD_PREPARE_BUFFER);
        let mut res_payload = vec![0u8; 5];
        res_payload[0] = 1;
        <LittleEndian as ByteOrder>::write_u32(&mut res_payload[1..5], 8);
        let res = ZKPacket::new(CMD_PREPARE_DATA, sid, packet.reply_id, res_payload);
        send_packet(&mut stream, &res);

        // _CMD_READ_BUFFER
        let packet = read_one_packet(&mut stream);
        assert_eq!(packet.command, _CMD_READ_BUFFER);

        // Send CMD_DATA with total_size = MAX_RESPONSE_SIZE + 1 (over limit)
        let mut data = Vec::new();
        data.write_i32::<LittleEndian>((MAX_RESPONSE_SIZE + 1) as i32)
            .unwrap();
        data.extend_from_slice(&[0xBB; 4]);
        let res = ZKPacket::new(CMD_DATA, sid, packet.reply_id, data);
        send_packet(&mut stream, &res);

        // CMD_FREE_DATA
        handle_ack(&mut stream, sid);
    });

    let mut zk = ZK::new("127.0.0.1", port);
    zk.connect(ZKProtocol::TCP).unwrap();

    let result = zk.get_templates();
    assert!(
        result.is_err(),
        "Should reject oversized template total_size"
    );
    let err = format!("{}", result.unwrap_err());
    assert!(
        err.contains("exceeds maximum"),
        "Error should mention exceeds maximum, got: {}",
        err
    );

    server.join().unwrap();
}

// =============================================================================
// Test 6: Zero-size response from read_with_buffer returns empty Vec
// =============================================================================

#[test]
fn test_zero_size_buffer_response() {
    let _ = env_logger::builder().is_test(true).try_init();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let sid = 2222;

        handle_connect(&mut stream, sid);
        handle_get_free_sizes(&mut stream, sid, 1, 0, 0);
        handle_tzadj(&mut stream, sid);

        // _CMD_PREPARE_BUFFER — return size = 0
        let packet = read_one_packet(&mut stream);
        assert_eq!(packet.command, _CMD_PREPARE_BUFFER);
        let mut res_payload = vec![0u8; 5];
        res_payload[0] = 1;
        <LittleEndian as ByteOrder>::write_u32(&mut res_payload[1..5], 0); // Zero size!
        let res = ZKPacket::new(CMD_ACK_OK, sid, packet.reply_id, res_payload);
        send_packet(&mut stream, &res);

        // CMD_FREE_DATA (sent by the zero-size guard)
        handle_ack(&mut stream, sid);
    });

    let mut zk = ZK::new("127.0.0.1", port);
    zk.connect(ZKProtocol::TCP).unwrap();

    // get_users internally calls read_with_buffer.
    // With size=0, it should return empty data (len <= 4), then return empty users.
    let users = zk.get_users().unwrap();
    assert_eq!(users.len(), 0, "Zero-size buffer should yield 0 users");

    server.join().unwrap();
}
