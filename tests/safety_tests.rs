use rustzk::constants::*;
use rustzk::protocol::{TCPWrapper, ZKPacket};
use rustzk::{ZKProtocol, ZK};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;
use byteorder::ByteOrder;

#[test]
fn test_max_discarded_packets_limit() {
    let _ = env_logger::builder().is_test(true).try_init();
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind mock server");
    let addr = listener.local_addr().unwrap();
    let port = addr.port();

    let server_handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("Failed to accept connection");
        let session_id = 1234;

        // 1. Handle Connect
        let mut header = [0u8; 8];
        stream.read_exact(&mut header).unwrap();
        let (length, _) = TCPWrapper::decode_header(&header).unwrap();
        let mut body = vec![0u8; length];
        stream.read_exact(&mut body).unwrap();
        let packet = ZKPacket::from_bytes(&body).unwrap();

        let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id(), vec![]);
        stream
            .write_all(&TCPWrapper::wrap(&res.to_bytes()))
            .unwrap();

        // 2. Handle CMD_GET_TIME (Client will trigger automated TZAdj sync first)
        stream.read_exact(&mut header).unwrap();
        let (length, _) = TCPWrapper::decode_header(&header).unwrap();
        let mut body = vec![0u8; length];
        stream.read_exact(&mut body).unwrap();
        let packet = ZKPacket::from_bytes_owned(body).unwrap();
        assert_eq!(packet.command(), CMD_OPTIONS_RRQ);
        let res = ZKPacket::new(
            CMD_ACK_OK,
            session_id,
            packet.reply_id(),
            b"TZAdj=7\0".to_vec(),
        );
        stream
            .write_all(&TCPWrapper::wrap(&res.to_bytes()))
            .unwrap();

        // 3. Handle the actual CMD_GET_TIME
        stream.read_exact(&mut header).unwrap();
        let (length, _) = TCPWrapper::decode_header(&header).unwrap();
        let mut body = vec![0u8; length];
        stream.read_exact(&mut body).unwrap();
        let packet = ZKPacket::from_bytes_owned(body).unwrap();
        assert_eq!(packet.command(), CMD_GET_TIME);

        // MOCK MALICIOUS DEVICE: Send MAX_DISCARDED_PACKETS + 1 stale packets
        for _ in 0..=MAX_DISCARDED_PACKETS {
            let stale_res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id() + 1, vec![]);
            stream
                .write_all(&TCPWrapper::wrap(&stale_res.to_bytes()))
                .unwrap();
        }
        stream.flush().unwrap();
    });

    let mut zk = ZK::new("127.0.0.1", port);
    zk.connect(ZKProtocol::TCP).unwrap();

    let result = zk.get_time();
    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    assert!(err_msg.contains("Too many discarded packets"));

    server_handle.join().unwrap();
}

#[test]
fn test_zk_drop_sends_exit_packet() {
    let _ = env_logger::builder().is_test(true).try_init();
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind mock server");
    let addr = listener.local_addr().unwrap();
    let port = addr.port();

    let server_handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("Failed to accept connection");
        let session_id = 1234;

        // 1. Handle Connect
        let mut header = [0u8; 8];
        stream.read_exact(&mut header).unwrap();
        let (length, _) = TCPWrapper::decode_header(&header).unwrap();
        let mut body = vec![0u8; length];
        stream.read_exact(&mut body).unwrap();
        let packet = ZKPacket::from_bytes(&body).unwrap();

        let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id(), vec![]);
        stream
            .write_all(&TCPWrapper::wrap(&res.to_bytes()))
            .unwrap();

        // 2. Read CMD_EXIT sent during Drop
        stream.read_exact(&mut header).unwrap();
        let (length, _) = TCPWrapper::decode_header(&header).unwrap();
        let mut body = vec![0u8; length];
        stream.read_exact(&mut body).unwrap();
        let exit_packet = ZKPacket::from_bytes(&body).unwrap();
        assert_eq!(exit_packet.command(), CMD_EXIT);
    });

    let mut zk = ZK::new("127.0.0.1", port);
    zk.connect(ZKProtocol::TCP).unwrap();
    assert!(zk.is_connected());

    // Drop the struct
    std::mem::drop(zk);

    server_handle.join().unwrap();
}

#[test]
fn test_calculate_checksum_on_raw_packet_buffer() {
    // A raw packet with a non-zero checksum: Command 1000, Session 0, Reply 65534, checksum 0xFC17
    let raw_packet = vec![0xE8, 0x03, 0x17, 0xFC, 0x00, 0x00, 0xFE, 0xFF];
    // Standalone calculate_checksum should skip the checksum field (bytes 2-3)
    // and compute 0xFC17, not including 0xFC17 in the calculation.
    let computed = rustzk::protocol::calculate_checksum(&raw_packet);
    assert_eq!(computed, 0xFC17);
}

#[test]
fn test_ambiguous_local_time_handling() {
    use chrono::NaiveDateTime;
    let naive = NaiveDateTime::parse_from_str("2026-02-19 09:16:41", "%Y-%m-%d %H:%M:%S").unwrap();
    
    // Construct an Attendance record with normal timezone
    let att = rustzk::models::Attendance::new(
        1,
        "101".to_string(),
        naive,
        1,
        0,
        420,
    );
    let dt = att.timestamp_fixed();
    assert!(dt.is_some());
    assert_eq!(dt.unwrap().to_rfc3339(), "2026-02-19T09:16:41+07:00");
}

#[test]
fn test_get_attendance_invalid_record_size() {
    let _ = env_logger::builder().is_test(true).try_init();
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind mock server");
    let addr = listener.local_addr().unwrap();
    let port = addr.port();

    let server_handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("Failed to accept connection");
        let session_id = 1234;

        // 1. Handle Connect
        let mut header = [0u8; 8];
        stream.read_exact(&mut header).unwrap();
        let (length, _) = TCPWrapper::decode_header(&header).unwrap();
        let mut body = vec![0u8; length];
        stream.read_exact(&mut body).unwrap();
        let packet = ZKPacket::from_bytes(&body).unwrap();
        let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id(), vec![]);
        stream.write_all(&TCPWrapper::wrap(&res.to_bytes())).unwrap();

        // 2. Handle CMD_GET_FREE_SIZES (read_sizes)
        stream.read_exact(&mut header).unwrap();
        let (length, _) = TCPWrapper::decode_header(&header).unwrap();
        let mut body = vec![0u8; length];
        stream.read_exact(&mut body).unwrap();
        let packet = ZKPacket::from_bytes_owned(body).unwrap();
        assert_eq!(packet.command(), CMD_GET_FREE_SIZES);

        // Mock response: 1 record
        let mut sizes_payload = vec![0u8; 80];
        // self.records is at bytes 32..36
        byteorder::LittleEndian::write_i32(&mut sizes_payload[32..36], 1);
        let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id(), sizes_payload);
        stream.write_all(&TCPWrapper::wrap(&res.to_bytes())).unwrap();

        // 3. Handle TZAdj Option Query
        stream.read_exact(&mut header).unwrap();
        let (length, _) = TCPWrapper::decode_header(&header).unwrap();
        let mut body = vec![0u8; length];
        stream.read_exact(&mut body).unwrap();
        let packet = ZKPacket::from_bytes_owned(body).unwrap();
        let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id(), b"TZAdj=7\0".to_vec());
        stream.write_all(&TCPWrapper::wrap(&res.to_bytes())).unwrap();

        // 4. Handle CMD_ATTLOG_RRQ (get_attendance)
        stream.read_exact(&mut header).unwrap();
        let (length, _) = TCPWrapper::decode_header(&header).unwrap();
        let mut body = vec![0u8; length];
        stream.read_exact(&mut body).unwrap();
        let packet = ZKPacket::from_bytes_owned(body).unwrap();
        assert_eq!(packet.command(), _CMD_PREPARE_BUFFER);

        // MOCK BAD DEVICE: Return CMD_DATA directly with 9 bytes payload.
        // First 4 bytes is total_size = 5. Remaining 5 bytes is data.
        // This makes record_size = 5 / 1 = 5 (invalid size).
        let mut bad_payload = vec![0u8; 9];
        byteorder::LittleEndian::write_u32(&mut bad_payload[0..4], 5);
        let res = ZKPacket::new(CMD_DATA, session_id, packet.reply_id(), bad_payload);
        stream.write_all(&TCPWrapper::wrap(&res.to_bytes())).unwrap();
    });

    let mut zk = ZK::new("127.0.0.1", port);
    zk.connect(ZKProtocol::TCP).unwrap();

    let result = zk.get_attendance();
    assert!(result.is_err());
    let err_msg = format!("{:?}", result.err().unwrap());
    assert!(err_msg.contains("Unsupported or invalid attendance record size"));

    server_handle.join().unwrap();
}
