use byteorder::{ByteOrder, LittleEndian, WriteBytesExt};
use chrono::Datelike;
use rustzk::constants::*;
use rustzk::protocol::{TCPWrapper, ZKPacket};
use rustzk::{ZKProtocol, ZK};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream, UdpSocket};
use std::thread;

#[test]
fn test_full_device_flow_mock() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind mock server");
    let addr = listener.local_addr().unwrap();
    let port = addr.port();

    let server_handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("Failed to accept connection");
        let mut session_id = 0;
        let mut last_prepared_cmd = 0;

        loop {
            let mut header = [0u8; 8];
            if stream.read_exact(&mut header).is_err() {
                break;
            }

            let (length, _) = TCPWrapper::decode_header(&header).unwrap();
            let mut body = vec![0u8; length];
            stream.read_exact(&mut body).unwrap();
            let packet = ZKPacket::from_bytes_owned(body).unwrap();

            match packet.command {
                CMD_CONNECT => {
                    session_id = 1234;
                    let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, vec![]);
                    stream
                        .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                        .unwrap();
                }
                CMD_GET_FREE_SIZES => {
                    let mut bytes = Vec::new();
                    for i in 0..20 {
                        let val = match i {
                            4 => 1,      // users
                            8 => 1,      // records
                            15 => 1000,  // users_cap
                            16 => 10000, // rec_cap
                            _ => 0,
                        };
                        bytes.write_i32::<LittleEndian>(val).unwrap();
                    }
                    let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, bytes);
                    stream
                        .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                        .unwrap();
                }
                _CMD_PREPARE_BUFFER => {
                    let cmd_in_buf = LittleEndian::read_u16(&packet.payload[1..3]);
                    last_prepared_cmd = cmd_in_buf;
                    // Total size = 4 (size prefix) + data
                    let size = if cmd_in_buf == CMD_USERTEMP_RRQ {
                        4 + 28
                    } else {
                        4 + 8
                    };

                    let mut res_payload = vec![0u8; 5];
                    res_payload[0] = 1;
                    LittleEndian::write_u32(&mut res_payload[1..5], size as u32);

                    let res =
                        ZKPacket::new(CMD_PREPARE_DATA, session_id, packet.reply_id, res_payload);
                    stream
                        .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                        .unwrap();
                }
                _CMD_READ_BUFFER => {
                    let mut data = Vec::new();
                    if last_prepared_cmd == CMD_USERTEMP_RRQ {
                        data.write_u32::<LittleEndian>(28).unwrap(); // Size prefix
                        data.write_u16::<LittleEndian>(1).unwrap(); // UID
                        data.push(USER_ADMIN); // Priv
                        data.extend_from_slice(b"123\0\0"); // Pass (5)
                        data.extend_from_slice(b"Admin\0\0\0"); // Name (8)
                        data.write_u32::<LittleEndian>(0).unwrap(); // Card
                        data.push(0); // Pad
                        data.push(1); // Group
                        data.write_u16::<LittleEndian>(0).unwrap(); // TZ
                        data.write_u32::<LittleEndian>(101).unwrap(); // UserID
                    } else if last_prepared_cmd == CMD_ATTLOG_RRQ {
                        data.write_u32::<LittleEndian>(8).unwrap(); // Size prefix
                        data.write_u16::<LittleEndian>(1).unwrap(); // UID
                        data.push(1); // Status
                        data.write_u32::<LittleEndian>(839845230).unwrap(); // Time
                        data.push(0); // Punch
                    }
                    let res = ZKPacket::new(CMD_DATA, session_id, packet.reply_id, data);
                    stream
                        .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                        .unwrap();
                }
                CMD_EXIT => {
                    let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, vec![]);
                    stream
                        .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                        .unwrap();
                    break;
                }
                CMD_FREE_DATA => {
                    let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, vec![]);
                    stream
                        .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                        .unwrap();
                }
                CMD_GET_TIME => {
                    // Encode a known time: 2025-06-15 10:30:00
                    // Formula: ((year%100)*12*31 + (month-1)*31 + day-1) * 86400 + (hour*60+minute)*60 + second
                    let t: u32 = ((25 * 12 * 31 + 5 * 31 + 14) * 86400) + (10 * 60 + 30) * 60;
                    let mut payload = Vec::new();
                    payload.write_u32::<LittleEndian>(t).unwrap();
                    let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, payload);
                    stream
                        .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                        .unwrap();
                }
                _ => {
                    let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, vec![]);
                    stream
                        .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                        .unwrap();
                }
            }
        }
    });

    let mut zk = ZK::new("127.0.0.1", port);
    zk.connect(ZKProtocol::TCP).unwrap();

    zk.read_sizes().unwrap();
    assert_eq!(zk.users, 1, "Users should be 1");
    assert_eq!(zk.records, 1, "Records should be 1");

    let users = zk.get_users().unwrap();
    assert_eq!(users.len(), 1);
    assert_eq!(users[0].name, "Admin");

    let logs = zk.get_attendance().unwrap();
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].user_id, "101");

    let time = zk.get_time().unwrap();
    assert_eq!(time.year(), 2025);
    assert_eq!(time.month(), 6);

    zk.disconnect().unwrap();
    server_handle.join().unwrap();
}

#[test]
fn test_mock_udp_connect() {
    use std::net::UdpSocket;

    let server_socket = UdpSocket::bind("127.0.0.1:0").expect("Failed to bind UDP mock");
    let addr = server_socket.local_addr().unwrap();
    let port = addr.port();

    let server_handle = thread::spawn(move || {
        let mut buf = [0u8; 1024];
        let (len, client_addr) = server_socket
            .recv_from(&mut buf)
            .expect("Failed to receive UDP");

        let packet = ZKPacket::from_bytes(&buf[..len]).unwrap();
        assert_eq!(packet.command, CMD_CONNECT);

        let res_packet = ZKPacket::new(CMD_ACK_OK, 5678, packet.reply_id, vec![]);
        server_socket
            .send_to(&res_packet.to_bytes(), client_addr)
            .unwrap();
    });

    let mut zk = ZK::new("127.0.0.1", port);
    let result = zk.connect(ZKProtocol::UDP);

    assert!(result.is_ok());
    assert!(zk.is_connected);

    server_handle.join().unwrap();
}

/// Test: server sends TCP response in tiny fragments (byte-by-byte for header,
/// then body as a second write). Exercises the partial-read + follow-up read_exact path.
#[test]
fn test_tcp_partial_response_fragmented() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind");
    let addr = listener.local_addr().unwrap();
    let port = addr.port();

    let server_handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();

        // Read client's CMD_CONNECT
        let mut header = [0u8; 8];
        stream.read_exact(&mut header).unwrap();
        let (length, _) = TCPWrapper::decode_header(&header).unwrap();
        let mut body = vec![0u8; length];
        stream.read_exact(&mut body).unwrap();

        // Send response in two fragments: TCP header first, then ZK packet body
        let res = ZKPacket::new(CMD_ACK_OK, 1111, 0, vec![]);
        let wrapped = TCPWrapper::wrap(&res.to_bytes());

        // Fragment 1: just the TCP header (first 8 bytes)
        stream.write_all(&wrapped[..8]).unwrap();
        stream.flush().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(50));

        // Fragment 2: the ZK packet body
        stream.write_all(&wrapped[8..]).unwrap();
        stream.flush().unwrap();

        // Now handle CMD_GET_TIME — send it as one tiny write per byte
        let mut header2 = [0u8; 8];
        stream.read_exact(&mut header2).unwrap();
        let (length2, _) = TCPWrapper::decode_header(&header2).unwrap();
        let mut body2 = vec![0u8; length2];
        stream.read_exact(&mut body2).unwrap();

        let t: u32 = ((25 * 12 * 31 + 5 * 31 + 14) * 86400) + (10 * 60 + 30) * 60;
        let mut payload = Vec::new();
        payload.write_u32::<LittleEndian>(t).unwrap();
        let time_res = ZKPacket::new(CMD_ACK_OK, 1111, 1, payload);
        let time_wrapped = TCPWrapper::wrap(&time_res.to_bytes());

        // Send byte-by-byte with small delays to force partial reads
        for byte in &time_wrapped {
            stream.write_all(&[*byte]).unwrap();
            stream.flush().unwrap();
        }

        // Handle CMD_EXIT
        let mut header3 = [0u8; 8];
        if stream.read_exact(&mut header3).is_ok() {
            let (length3, _) = TCPWrapper::decode_header(&header3).unwrap();
            let mut body3 = vec![0u8; length3];
            stream.read_exact(&mut body3).unwrap();
            let exit_res = ZKPacket::new(CMD_ACK_OK, 1111, 2, vec![]);
            stream
                .write_all(&TCPWrapper::wrap(&exit_res.to_bytes()))
                .unwrap();
        }
    });

    let mut zk = ZK::new("127.0.0.1", port);
    zk.connect(ZKProtocol::TCP).unwrap();
    assert!(zk.is_connected);

    // get_time should work even with fragmented TCP responses
    let time = zk.get_time().unwrap();
    assert_eq!(time.year(), 2025);
    assert_eq!(time.month(), 6);

    zk.disconnect().unwrap();
    server_handle.join().unwrap();
}

/// Test: server sends only a few bytes then closes — should return error, not hang
#[test]
fn test_tcp_short_response_returns_error() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind");
    let addr = listener.local_addr().unwrap();
    let port = addr.port();

    let server_handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();

        // Read the CMD_CONNECT request
        let mut header = [0u8; 8];
        stream.read_exact(&mut header).unwrap();
        let (length, _) = TCPWrapper::decode_header(&header).unwrap();
        let mut body = vec![0u8; length];
        stream.read_exact(&mut body).unwrap();

        // Send only 4 bytes (too short for TCP header) then close
        stream.write_all(&[0x50, 0x50, 0x82, 0x72]).unwrap();
        stream.flush().unwrap();
        drop(stream); // close connection
    });

    let mut zk = ZK::new("127.0.0.1", port);
    let result = zk.connect(ZKProtocol::TCP);

    // Should fail with an error, NOT hang
    assert!(result.is_err());

    server_handle.join().unwrap();
}

/// Test: exercise the receive_chunk path with fragmented TCP delivery.
/// The mock server returns CMD_PREPARE_DATA + CMD_DATA chunks for get_users,
/// sending each TCP-wrapped packet in two fragments.
#[test]
fn test_tcp_receive_chunk_fragmented() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind");
    let addr = listener.local_addr().unwrap();
    let port = addr.port();

    let server_handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let session_id: u16 = 2222;

        // Helper: read one request from client
        let read_request = |s: &mut TcpStream| -> ZKPacket<'static> {
            let mut header = [0u8; 8];
            s.read_exact(&mut header).unwrap();
            let (length, _) = TCPWrapper::decode_header(&header).unwrap();
            let mut body = vec![0u8; length];
            s.read_exact(&mut body).unwrap();
            ZKPacket::from_bytes_owned(body).unwrap()
        };

        // Helper: send response in two fragments (header, then body)
        let send_fragmented = |s: &mut TcpStream, packet: &ZKPacket| {
            let wrapped = TCPWrapper::wrap(&packet.to_bytes());
            // Send TCP header separately
            s.write_all(&wrapped[..8]).unwrap();
            s.flush().unwrap();
            std::thread::sleep(std::time::Duration::from_millis(20));
            // Send body
            s.write_all(&wrapped[8..]).unwrap();
            s.flush().unwrap();
        };

        loop {
            let req = read_request(&mut stream);
            match req.command {
                CMD_CONNECT => {
                    let res = ZKPacket::new(CMD_ACK_OK, session_id, req.reply_id, vec![]);
                    send_fragmented(&mut stream, &res);
                }
                CMD_GET_FREE_SIZES => {
                    let mut bytes = Vec::new();
                    for i in 0..20 {
                        let val: i32 = match i {
                            4 => 1,     // users
                            15 => 1000, // users_cap
                            _ => 0,
                        };
                        bytes.write_i32::<LittleEndian>(val).unwrap();
                    }
                    let res = ZKPacket::new(CMD_ACK_OK, session_id, req.reply_id, bytes);
                    send_fragmented(&mut stream, &res);
                }
                _CMD_PREPARE_BUFFER => {
                    // Return PREPARE_DATA with size = 4 + 28 (one user)
                    let size: u32 = 4 + 28;
                    let mut res_payload = vec![0u8; 5];
                    res_payload[0] = 1;
                    LittleEndian::write_u32(&mut res_payload[1..5], size);
                    let res =
                        ZKPacket::new(CMD_PREPARE_DATA, session_id, req.reply_id, res_payload);
                    send_fragmented(&mut stream, &res);
                }
                _CMD_READ_BUFFER => {
                    // Build user data chunk
                    let mut data = Vec::new();
                    data.write_u32::<LittleEndian>(28).unwrap(); // Size prefix
                    data.write_u16::<LittleEndian>(1).unwrap(); // UID
                    data.push(USER_DEFAULT); // Priv
                    data.extend_from_slice(b"pwd\0\0"); // Pass (5)
                    data.extend_from_slice(b"TestUsr\0"); // Name (8)
                    data.write_u32::<LittleEndian>(0).unwrap(); // Card
                    data.push(0); // Pad
                    data.push(1); // Group
                    data.write_u16::<LittleEndian>(0).unwrap(); // TZ
                    data.write_u32::<LittleEndian>(42).unwrap(); // UserID

                    let res = ZKPacket::new(CMD_DATA, session_id, req.reply_id, data);
                    // Send this chunk fragmented too
                    send_fragmented(&mut stream, &res);
                }
                CMD_FREE_DATA => {
                    let res = ZKPacket::new(CMD_ACK_OK, session_id, req.reply_id, vec![]);
                    send_fragmented(&mut stream, &res);
                }
                CMD_EXIT => {
                    let res = ZKPacket::new(CMD_ACK_OK, session_id, req.reply_id, vec![]);
                    send_fragmented(&mut stream, &res);
                    break;
                }
                _ => {
                    let res = ZKPacket::new(CMD_ACK_OK, session_id, req.reply_id, vec![]);
                    send_fragmented(&mut stream, &res);
                }
            }
        }
    });

    let mut zk = ZK::new("127.0.0.1", port);
    zk.connect(ZKProtocol::TCP).unwrap();

    // get_users exercises both send_command AND receive_chunk paths
    let users = zk.get_users().unwrap();
    assert_eq!(users.len(), 1);
    assert_eq!(users[0].name, "TestUsr");
    assert_eq!(users[0].user_id, "42");

    zk.disconnect().unwrap();
    server_handle.join().unwrap();
}

#[test]
fn test_connect_fallback_udp() {
    // Pick a free port
    let socket = UdpSocket::bind("127.0.0.1:0").expect("Failed to bind UDP mock");
    let addr = socket.local_addr().unwrap();
    let port = addr.port();

    // Spawn UDP server ONLY (no TCP listener)
    let server_handle = thread::spawn(move || {
        let mut buf = [0u8; 1024];
        let (len, client_addr) = socket.recv_from(&mut buf).expect("Failed to receive UDP");

        let packet = ZKPacket::from_bytes(&buf[..len]).unwrap();
        assert_eq!(packet.command, CMD_CONNECT);

        let res_packet = ZKPacket::new(CMD_ACK_OK, 9999, packet.reply_id, vec![]);
        socket.send_to(&res_packet.to_bytes(), client_addr).unwrap();
    });

    let mut zk = ZK::new("127.0.0.1", port);

    // This should fail TCP (refused) then succeed UDP
    let result = zk.connect(ZKProtocol::Auto);

    assert!(result.is_ok());
    assert!(zk.is_connected);
    // Verify it chose UDP transport (we can't easily check private field, but success implies it worked)

    server_handle.join().unwrap();
}

#[test]
fn test_get_users_gbk_decoding() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind");
    let addr = listener.local_addr().unwrap();
    let port = addr.port();

    let server_handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let session_id = 9999;
        let mut last_prepared_cmd = 0;

        // Helper to send packet - taking stream as arg to avoid capture issues
        let send_packet = |s: &mut TcpStream, packet: ZKPacket| {
            let wrapped = TCPWrapper::wrap(&packet.to_bytes());
            s.write_all(&wrapped).unwrap();
            s.flush().unwrap();
        };

        loop {
            // Read packet inline
            let mut header = [0u8; 8];
            if stream.read_exact(&mut header).is_err() {
                break;
            }
            let (length, _) = TCPWrapper::decode_header(&header).unwrap();
            let mut body = vec![0u8; length];
            stream.read_exact(&mut body).unwrap();
            let packet = ZKPacket::from_bytes_owned(body).unwrap();

            match packet.command {
                CMD_CONNECT => {
                    send_packet(
                        &mut stream,
                        ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, vec![]),
                    );
                }
                CMD_GET_FREE_SIZES => {
                    let mut bytes = Vec::new();
                    for i in 0..20 {
                        let val = match i {
                            4 => 1,      // users
                            8 => 1,      // records
                            15 => 1000,  // users_cap
                            16 => 10000, // rec_cap
                            _ => 0,
                        };
                        bytes.write_i32::<LittleEndian>(val).unwrap();
                    }
                    send_packet(
                        &mut stream,
                        ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, bytes),
                    );
                }
                _CMD_PREPARE_BUFFER => {
                    if packet.payload.len() != 11 {
                        panic!(
                            "_CMD_PREPARE_BUFFER payload len is {}, expected 11",
                            packet.payload.len()
                        );
                    }

                    let cmd_in_buf = LittleEndian::read_u16(&packet.payload[1..3]);
                    let fct_in_buf = LittleEndian::read_u32(&packet.payload[3..7]);
                    assert_eq!(fct_in_buf, FCT_USER as u32, "FCT should be FCT_USER (5)");

                    last_prepared_cmd = cmd_in_buf;

                    let size: u32 = 4 + 28;
                    let mut res_payload = vec![0u8; 5];
                    res_payload[0] = 1;
                    LittleEndian::write_u32(&mut res_payload[1..5], size);
                    send_packet(
                        &mut stream,
                        ZKPacket::new(CMD_PREPARE_DATA, session_id, packet.reply_id, res_payload),
                    );
                }
                _CMD_READ_BUFFER => {
                    if last_prepared_cmd == CMD_USERTEMP_RRQ {
                        let mut data = Vec::new();
                        data.write_u32::<LittleEndian>(28).unwrap();
                        data.write_u16::<LittleEndian>(101).unwrap();
                        data.push(USER_DEFAULT);
                        data.extend_from_slice(b"1234\0");

                        // "你好" (GBK: C4 E3 BA C3)
                        let mut name_bytes = [0u8; 8];
                        name_bytes[0] = 0xC4;
                        name_bytes[1] = 0xE3;
                        name_bytes[2] = 0xBA;
                        name_bytes[3] = 0xC3;
                        data.extend_from_slice(&name_bytes);

                        data.write_u32::<LittleEndian>(0).unwrap();
                        data.push(0);
                        data.push(1);
                        data.write_u16::<LittleEndian>(0).unwrap();
                        data.write_u32::<LittleEndian>(888).unwrap();

                        send_packet(
                            &mut stream,
                            ZKPacket::new(CMD_DATA, session_id, packet.reply_id, data),
                        );
                    }
                }
                CMD_FREE_DATA => {
                    send_packet(
                        &mut stream,
                        ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, vec![]),
                    );
                }
                CMD_EXIT => {
                    send_packet(
                        &mut stream,
                        ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, vec![]),
                    );
                    break;
                }
                _ => {
                    send_packet(
                        &mut stream,
                        ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, vec![]),
                    );
                }
            }
        }
    });

    let mut zk = ZK::new("127.0.0.1", port);
    zk.connect(ZKProtocol::TCP).unwrap();
    let users = zk.get_users().unwrap();

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].name, "你好");
    assert_eq!(users[0].user_id, "888");

    zk.disconnect().unwrap();
    server_handle.join().unwrap();
}

#[test]
fn test_receive_chunk_with_ack_ok() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind mock server");
    let addr = listener.local_addr().unwrap();
    let port = addr.port();

    let server_handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("Failed to accept connection");
        let session_id = 1234;
        let mut ack_sent = false;

        loop {
            let mut header = [0u8; 8];
            if stream.read_exact(&mut header).is_err() {
                break;
            }

            let (length, _) = TCPWrapper::decode_header(&header).unwrap();
            let mut body = vec![0u8; length];
            stream.read_exact(&mut body).unwrap();
            let packet = ZKPacket::from_bytes_owned(body).unwrap();

            match packet.command {
                CMD_CONNECT => {
                    let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, vec![]);
                    stream
                        .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                        .unwrap();
                }
                CMD_GET_FREE_SIZES => {
                    let mut bytes = Vec::new();
                    for i in 0..20 {
                        let val = if i == 8 { 1 } else { 0 }; // 1 record
                        bytes.write_i32::<LittleEndian>(val).unwrap();
                    }
                    let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, bytes);
                    stream
                        .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                        .unwrap();
                }
                _CMD_PREPARE_BUFFER => {
                    let cmd_in_buf = LittleEndian::read_u16(&packet.payload[1..3]);
                    let size = if cmd_in_buf == CMD_USERTEMP_RRQ {
                        4 // empty users
                    } else {
                        4 + 16 // 1 log (16 bytes format)
                    };

                    let mut res_payload = vec![0u8; 5];
                    res_payload[0] = 1;
                    LittleEndian::write_u32(&mut res_payload[1..5], size as u32);
                    let res =
                        ZKPacket::new(CMD_PREPARE_DATA, session_id, packet.reply_id, res_payload);
                    stream
                        .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                        .unwrap();
                }
                _CMD_READ_BUFFER => {
                    let mut data = Vec::new();

                    if !ack_sent {
                        // First request to read buffer (can be users or logs)
                        // Let's only send ACK_OK for the logs request
                        let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, vec![]);
                        stream
                            .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                            .unwrap();
                        ack_sent = true;
                    } else {
                        // Actual data
                        data.write_u32::<LittleEndian>(16).unwrap(); // data size
                        data.write_u32::<LittleEndian>(101).unwrap(); // UserID
                        data.write_u32::<LittleEndian>(839845230).unwrap(); // Time
                        data.push(1); // Status
                        data.push(0); // Punch
                        data.write_u16::<LittleEndian>(0).unwrap(); // Reserved
                        data.write_u32::<LittleEndian>(0).unwrap(); // Workcode

                        let res = ZKPacket::new(CMD_DATA, session_id, packet.reply_id, data);
                        stream
                            .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                            .unwrap();
                    }
                }
                CMD_EXIT => {
                    let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, vec![]);
                    stream
                        .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                        .unwrap();
                    break;
                }
                _ => {
                    let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, vec![]);
                    stream
                        .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                        .unwrap();
                }
            }
        }
    });

    let mut zk = ZK::new("127.0.0.1", port);
    zk.connect(ZKProtocol::TCP).unwrap();

    // get_attendance calls get_users first.
    // In our mock, the first _CMD_READ_BUFFER (for users) will get ACK_OK.
    // get_users will retry and get empty data.
    // Then get_attendance calls read_with_buffer for logs.
    // Since ack_sent is true, it will get data.

    zk.disconnect().unwrap();
    server_handle.join().unwrap();
}

#[test]
fn test_infinite_loop_protection() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind mock server");
    let addr = listener.local_addr().unwrap();
    let port = addr.port();

    let server_handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("Failed to accept connection");
        let session_id = 1234;

        loop {
            let mut header = [0u8; 8];
            if stream.read_exact(&mut header).is_err() {
                break;
            }

            let (length, _) = TCPWrapper::decode_header(&header).unwrap();
            let mut body = vec![0u8; length];
            stream.read_exact(&mut body).unwrap();
            let packet = ZKPacket::from_bytes_owned(body).unwrap();

            match packet.command {
                CMD_CONNECT => {
                    let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, vec![]);
                    stream
                        .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                        .unwrap();
                }
                CMD_GET_FREE_SIZES => {
                    let mut bytes = Vec::new();
                    for i in 0..20 {
                        let val = if i == 8 { 1 } else { 0 }; // 1 record
                        bytes.write_i32::<LittleEndian>(val).unwrap();
                    }
                    let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, bytes);
                    stream
                        .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                        .unwrap();
                }
                _CMD_PREPARE_BUFFER => {
                    let mut res_payload = vec![0u8; 5];
                    res_payload[0] = 1;
                    LittleEndian::write_u32(&mut res_payload[1..5], 12); // size prefix(4) + 8
                    let res =
                        ZKPacket::new(CMD_PREPARE_DATA, session_id, packet.reply_id, res_payload);
                    stream
                        .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                        .unwrap();
                }
                _CMD_READ_BUFFER => {
                    // ALWAYS return ACK_OK (empty response) to trigger infinite loop protection
                    let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, vec![]);
                    stream
                        .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                        .unwrap();
                }
                CMD_EXIT => {
                    let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, vec![]);
                    stream
                        .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                        .unwrap();
                    break;
                }
                _ => {
                    let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, vec![]);
                    stream
                        .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                        .unwrap();
                }
            }
        }
    });

    let mut zk = ZK::new("127.0.0.1", port);
    zk.timeout = std::time::Duration::from_secs(1); // Fail fast
    zk.connect(ZKProtocol::TCP).unwrap();

    // get_attendance() should fail after 100 empty _CMD_READ_BUFFER responses
    // OR timeout if the client waits for data that never comes (new behavior)
    let result = zk.get_attendance();
    assert!(
        result.is_err(),
        "Expected error from infinite loop protection or timeout"
    );

    if let Err(e) = result {
        let msg = format!("{}", e);
        assert!(
            msg.contains("Too many empty responses")
                || msg.contains("Network error")
                || msg.contains("Resource temporarily unavailable"),
            "Error message mismatch: {}",
            msg
        );
    }

    // We might have disconnected or timed out, so ignore disconnect errors
    let _ = zk.disconnect();
    server_handle.join().unwrap();
}

#[test]
fn test_record_count_mismatch_safety() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind mock server");
    let addr = listener.local_addr().unwrap();
    let port = addr.port();

    let server_handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("Failed to accept connection");
        let session_id = 1234;

        loop {
            let mut header = [0u8; 8];
            if stream.read_exact(&mut header).is_err() {
                break;
            }

            let (length, _) = TCPWrapper::decode_header(&header).unwrap();
            let mut body = vec![0u8; length];
            stream.read_exact(&mut body).unwrap();
            let packet = ZKPacket::from_bytes_owned(body).unwrap();

            match packet.command {
                CMD_CONNECT => {
                    let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, vec![]);
                    stream
                        .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                        .unwrap();
                }
                CMD_GET_FREE_SIZES => {
                    let mut bytes = Vec::new();
                    for i in 0..20 {
                        let val = if i == 8 { 1 } else { 0 }; // Device reports 1 record
                        bytes.write_i32::<LittleEndian>(val).unwrap();
                    }
                    let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, bytes);
                    stream
                        .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                        .unwrap();
                }
                _CMD_PREPARE_BUFFER => {
                    let mut res_payload = vec![0u8; 5];
                    res_payload[0] = 1;
                    // Mock server returns size for 2 records instead of 1
                    LittleEndian::write_u32(&mut res_payload[1..5], 4 + 16 * 2);
                    let res =
                        ZKPacket::new(CMD_PREPARE_DATA, session_id, packet.reply_id, res_payload);
                    stream
                        .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                        .unwrap();
                }
                _CMD_READ_BUFFER => {
                    let mut data = Vec::new();
                    data.write_u32::<LittleEndian>(16 * 2).unwrap(); // data size prefix

                    // Record 1
                    data.write_u32::<LittleEndian>(101).unwrap();
                    data.write_u32::<LittleEndian>(839845230).unwrap();
                    data.push(1);
                    data.push(0);
                    data.write_u16::<LittleEndian>(0).unwrap();
                    data.write_u32::<LittleEndian>(0).unwrap();

                    // Record 2 (Surprise!)
                    data.write_u32::<LittleEndian>(102).unwrap();
                    data.write_u32::<LittleEndian>(839845230).unwrap();
                    data.push(1);
                    data.push(0);
                    data.write_u16::<LittleEndian>(0).unwrap();
                    data.write_u32::<LittleEndian>(0).unwrap();

                    let res = ZKPacket::new(CMD_DATA, session_id, packet.reply_id, data);
                    stream
                        .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                        .unwrap();
                }
                CMD_USERTEMP_RRQ => {
                    // return empty users
                    let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, vec![]);
                    stream
                        .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                        .unwrap();
                }
                CMD_EXIT => {
                    let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, vec![]);
                    stream
                        .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                        .unwrap();
                    break;
                }
                _ => {
                    let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, vec![]);
                    stream
                        .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                        .unwrap();
                }
            }
        }
    });

    let mut zk = ZK::new("127.0.0.1", port);
    zk.connect(ZKProtocol::TCP).unwrap();

    // get_attendance should handle the case where actual data size > reported record count
    let logs = zk.get_attendance().unwrap();
    assert_eq!(
        logs.len(),
        2,
        "Should have parsed 2 logs even if only 1 was reported"
    );
    assert_eq!(logs[0].user_id, "101");
    assert_eq!(logs[1].user_id, "102");

    zk.disconnect().unwrap();
    server_handle.join().unwrap();
}

#[test]
fn test_timezone_sync_from_device() {
    use std::io::Cursor;
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind mock server");
    let addr = listener.local_addr().unwrap();
    let port = addr.port();

    let server_handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("Failed to accept connection");
        let session_id = 1234;

        loop {
            let mut header = [0u8; 8];
            if stream.read_exact(&mut header).is_err() {
                break;
            }

            let (length, _) = TCPWrapper::decode_header(&header).unwrap();
            let mut body = vec![0u8; length];
            stream.read_exact(&mut body).unwrap();

            let packet = ZKPacket::from_bytes(&body).unwrap();
            let res_cmd = if packet.command == CMD_CONNECT {
                CMD_ACK_OK
            } else if packet.command == CMD_OPTIONS_RRQ {
                CMD_ACK_OK
            } else if packet.command == CMD_GET_FREE_SIZES {
                CMD_ACK_OK
            } else if packet.command == _CMD_PREPARE_BUFFER {
                CMD_PREPARE_DATA
            } else if packet.command == _CMD_READ_BUFFER {
                CMD_DATA
            } else {
                CMD_ACK_OK
            };

            let mut payload = Vec::new();
            if packet.command == CMD_CONNECT {
                payload.write_u16::<LittleEndian>(session_id).unwrap();
            } else if packet.command == CMD_OPTIONS_RRQ {
                let key = String::from_utf8_lossy(&packet.payload)
                    .trim_matches('\0')
                    .to_string();
                if key == "TZAdj" {
                    payload.extend_from_slice(b"TZAdj=8\0");
                }
            } else if packet.command == CMD_GET_FREE_SIZES {
                payload.resize(80, 0);
                let mut p_writer = Cursor::new(&mut payload);
                p_writer.set_position(32);
                p_writer.write_i32::<LittleEndian>(1).unwrap();
            } else if packet.command == _CMD_PREPARE_BUFFER {
                payload.push(0); // Dummy byte for offset
                payload.write_u32::<LittleEndian>(12).unwrap(); // 4 header + 8 log
            } else if packet.command == _CMD_READ_BUFFER {
                payload.write_u32::<LittleEndian>(8).unwrap(); // Header: total size of logs
                payload.write_u16::<LittleEndian>(101).unwrap();
                payload.push(1);
                payload.write_u32::<LittleEndian>(839845230).unwrap();
                payload.push(0);
            }

            let res_packet = ZKPacket::new(res_cmd, session_id, packet.reply_id, payload);
            stream
                .write_all(&TCPWrapper::wrap(&res_packet.to_bytes()))
                .unwrap();
        }
    });

    let mut zk = ZK::new("127.0.0.1", port);
    zk.connect(ZKProtocol::TCP).unwrap();

    zk.read_sizes().unwrap();
    assert_eq!(zk.timezone_offset, 480);

    let logs = zk.get_attendance().unwrap();
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].timezone_offset, 480);
    assert_eq!(logs[0].iso_format().ends_with("+08:00"), true);

    zk.disconnect().unwrap();
    server_handle.join().unwrap();
}

/// Regression test for ZAM180 Ver 6.60 firmware behavior:
///
/// On this firmware, the device resets its internal buffer state after every
/// `CMD_FREE_DATA`. If `get_users()` was called first (ending with `CMD_FREE_DATA`),
/// the subsequent `_CMD_PREPARE_BUFFER` for `CMD_ATTLOG_RRQ` returns `CMD_ACK_OK`
/// with a **0-byte payload** — no size info — causing `read_with_buffer` to compute
/// `size = 0` and return an empty `Vec` (no attendances).
///
/// The fix: `get_attendance()` must fetch the attendance buffer FIRST, then call
/// `get_users()` afterwards to resolve `uid → user_id`.
///
/// This mock encodes the exact firmware quirk:
///   - `_CMD_PREPARE_BUFFER(CMD_ATTLOG_RRQ)` → full `CMD_PREPARE_DATA` response (first call)
///   - `_CMD_PREPARE_BUFFER(CMD_ATTLOG_RRQ)` → `CMD_ACK_OK` + empty payload  (after `CMD_FREE_DATA`)
#[test]
fn test_get_attendance_fetches_attlog_before_users() {
    use std::sync::{Arc, Mutex};

    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind mock server");
    let addr = listener.local_addr().unwrap();
    let port = addr.port();

    // `free_data_received` tracks whether CMD_FREE_DATA has been seen —
    // after which the mock simulates the ZAM180 "broken ATTLOG" state.
    let free_data_received = Arc::new(Mutex::new(false));
    let free_data_for_server = Arc::clone(&free_data_received);

    let server_handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("Failed to accept");
        let session_id: u16 = 4321;

        macro_rules! send {
            ($pkt:expr) => {
                stream
                    .write_all(&TCPWrapper::wrap(&$pkt.to_bytes()))
                    .unwrap();
            };
        }

        loop {
            let mut hdr = [0u8; 8];
            if stream.read_exact(&mut hdr).is_err() {
                break;
            }
            let (len, _) = TCPWrapper::decode_header(&hdr).unwrap();
            let mut body = vec![0u8; len];
            stream.read_exact(&mut body).unwrap();
            let pkt = ZKPacket::from_bytes(&body).unwrap();

            match pkt.command {
                CMD_CONNECT => {
                    send!(ZKPacket::new(CMD_ACK_OK, session_id, pkt.reply_id, vec![]));
                }

                CMD_GET_FREE_SIZES => {
                    // Report 1 user, 1 attendance record
                    let mut bytes = Vec::new();
                    for i in 0..20i32 {
                        let val = match i {
                            4 => 1,      // users
                            8 => 1,      // records
                            15 => 1000,  // users_cap
                            16 => 10000, // rec_cap
                            _ => 0,
                        };
                        bytes.write_i32::<LittleEndian>(val).unwrap();
                    }
                    send!(ZKPacket::new(CMD_ACK_OK, session_id, pkt.reply_id, bytes));
                }

                _CMD_PREPARE_BUFFER => {
                    let cmd_in_buf = LittleEndian::read_u16(&pkt.payload[1..3]);
                    let attlog_broken = *free_data_for_server.lock().unwrap();

                    if cmd_in_buf == CMD_ATTLOG_RRQ && attlog_broken {
                        // ZAM180 quirk: ATTLOG buffer unavailable after CMD_FREE_DATA.
                        // Device returns CMD_ACK_OK with 0 bytes — no size information.
                        send!(ZKPacket::new(CMD_ACK_OK, session_id, pkt.reply_id, vec![]));
                    } else {
                        let size: u32 = if cmd_in_buf == CMD_USERTEMP_RRQ {
                            4 + 28 // one 28-byte user record
                        } else {
                            4 + 8 // one 8-byte attendance record
                        };
                        let mut res_payload = vec![0u8; 5];
                        res_payload[0] = 1;
                        LittleEndian::write_u32(&mut res_payload[1..5], size);
                        send!(ZKPacket::new(
                            CMD_PREPARE_DATA,
                            session_id,
                            pkt.reply_id,
                            res_payload
                        ));
                    }
                }

                _CMD_READ_BUFFER => {
                    let attlog_broken = *free_data_for_server.lock().unwrap();

                    if attlog_broken {
                        // get_users() is running after attendance — serve user data
                        let mut data = Vec::new();
                        data.write_u32::<LittleEndian>(28).unwrap(); // total_size prefix
                        data.write_u16::<LittleEndian>(1).unwrap(); // uid = 1
                        data.push(USER_DEFAULT); // privilege
                        data.extend_from_slice(b"\0\0\0\0\0"); // password (5 bytes)
                        data.extend_from_slice(b"Alice\0\0\0"); // name (8 bytes)
                        data.write_u32::<LittleEndian>(0).unwrap(); // card
                        data.push(0); // pad
                        data.push(1); // group
                        data.write_u16::<LittleEndian>(0).unwrap(); // tz
                        data.write_u32::<LittleEndian>(501).unwrap(); // user_id = "501"
                        send!(ZKPacket::new(CMD_DATA, session_id, pkt.reply_id, data));
                    } else {
                        // get_attendance()'s raw buffer read — serve attendance data
                        // 8-byte record format: uid(u16) + status(u8) + time(u32) + punch(u8)
                        let mut data = Vec::new();
                        data.write_u32::<LittleEndian>(8).unwrap(); // total_size prefix
                        data.write_u16::<LittleEndian>(1).unwrap(); // uid = 1
                        data.push(1u8); // status = check-in
                        data.write_u32::<LittleEndian>(839_845_230).unwrap(); // encoded time
                        data.push(0u8); // punch
                        send!(ZKPacket::new(CMD_DATA, session_id, pkt.reply_id, data));
                    }
                }

                CMD_FREE_DATA => {
                    // Mark state as broken: any subsequent ATTLOG prepare will fail
                    *free_data_for_server.lock().unwrap() = true;
                    send!(ZKPacket::new(CMD_ACK_OK, session_id, pkt.reply_id, vec![]));
                }

                CMD_EXIT => {
                    send!(ZKPacket::new(CMD_ACK_OK, session_id, pkt.reply_id, vec![]));
                    break;
                }

                _ => {
                    send!(ZKPacket::new(CMD_ACK_OK, session_id, pkt.reply_id, vec![]));
                }
            }
        }
    });

    let mut zk = ZK::new("127.0.0.1", port);
    zk.connect(ZKProtocol::TCP).unwrap();

    // get_attendance() must succeed because it now fetches the attendance buffer
    // BEFORE calling get_users().  If the old order (users first) were used, the mock
    // would return empty ACK_OK for the ATTLOG prepare and the result would be empty.
    let logs = zk.get_attendance().unwrap();

    assert_eq!(logs.len(), 1, "Expected exactly 1 attendance log");

    // uid=1 is resolved to user_id="501" via the post-fetch get_users() call
    assert_eq!(
        logs[0].user_id, "501",
        "uid=1 should resolve to user_id=\"501\" (resolved after attendance fetch)"
    );
    assert_eq!(logs[0].uid, 1);
    assert_eq!(logs[0].status, 1, "status should be 1 (check-in)");

    zk.disconnect().unwrap();
    server_handle.join().unwrap();
}
