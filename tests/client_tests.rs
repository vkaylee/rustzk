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
            let packet = ZKPacket::from_bytes(&body).unwrap();

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
        let mut read_request = |s: &mut TcpStream| -> ZKPacket {
            let mut header = [0u8; 8];
            s.read_exact(&mut header).unwrap();
            let (length, _) = TCPWrapper::decode_header(&header).unwrap();
            let mut body = vec![0u8; length];
            s.read_exact(&mut body).unwrap();
            ZKPacket::from_bytes(&body).unwrap()
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
        let mut send_packet = |s: &mut TcpStream, packet: ZKPacket| {
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
            let packet = ZKPacket::from_bytes(&body).unwrap();

            match packet.command {
                CMD_CONNECT => {
                    send_packet(&mut stream, ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, vec![]));
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
                    send_packet(&mut stream, ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, bytes));
                }
                _CMD_PREPARE_BUFFER => {
                    if packet.payload.len() != 11 {
                        panic!("_CMD_PREPARE_BUFFER payload len is {}, expected 11", packet.payload.len());
                    }

                    let cmd_in_buf = LittleEndian::read_u16(&packet.payload[1..3]);
                    let fct_in_buf = LittleEndian::read_u32(&packet.payload[3..7]);
                    assert_eq!(fct_in_buf, FCT_USER as u32, "FCT should be FCT_USER (5)");

                    last_prepared_cmd = cmd_in_buf;
                    
                    let size: u32 = 4 + 28; 
                    let mut res_payload = vec![0u8; 5];
                    res_payload[0] = 1;
                    LittleEndian::write_u32(&mut res_payload[1..5], size);
                    send_packet(&mut stream, ZKPacket::new(CMD_PREPARE_DATA, session_id, packet.reply_id, res_payload));
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

                        send_packet(&mut stream, ZKPacket::new(CMD_DATA, session_id, packet.reply_id, data));
                    }
                }
                CMD_FREE_DATA => {
                    send_packet(&mut stream, ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, vec![]));
                }
                CMD_EXIT => {
                    send_packet(&mut stream, ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, vec![]));
                    break;
                }
                _ => {
                    send_packet(&mut stream, ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, vec![]));
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
