use byteorder::{ByteOrder, LittleEndian, WriteBytesExt};
use rustzk::constants::*;
use rustzk::protocol::{TCPWrapper, ZKPacket};
use rustzk::{ZKProtocol, ZK};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

/// Helper: read one TCP request from client
fn read_request(stream: &mut TcpStream) -> ZKPacket<'static> {
    let mut header = [0u8; 8];
    stream.read_exact(&mut header).unwrap();
    let (length, _) = TCPWrapper::decode_header(&header).unwrap();
    let mut body = vec![0u8; length];
    stream.read_exact(&mut body).unwrap();
    ZKPacket::from_bytes_owned(body).unwrap()
}

/// Helper: send a TCP response
fn send_response(stream: &mut TcpStream, packet: &ZKPacket) {
    stream
        .write_all(&TCPWrapper::wrap(&packet.to_bytes()))
        .unwrap();
}

/// Verifies send_command works with &[] (empty slice) for commands with no payload.
/// Uses a loop-based server to handle sync_timezone calls from read_sizes().
#[test]
fn test_send_command_empty_slice() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().unwrap().port();

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let session_id = 1234;
        let mut empty_payload_cmds: Vec<u16> = Vec::new();

        loop {
            let req = read_request(&mut stream);

            // Track commands that should have empty payloads
            let empty_payload_commands = [
                CMD_CONNECT,
                CMD_GET_FREE_SIZES,
                CMD_GET_VERSION,
                CMD_GET_TIME,
                CMD_REFRESHDATA,
                CMD_EXIT,
            ];
            if empty_payload_commands.contains(&req.command) {
                assert!(
                    req.payload.is_empty(),
                    "Command 0x{:X} should have empty payload but got {} bytes",
                    req.command,
                    req.payload.len()
                );
                empty_payload_cmds.push(req.command);
            }

            match req.command {
                CMD_GET_FREE_SIZES => {
                    let mut bytes = Vec::new();
                    for _ in 0..20 {
                        bytes.write_i32::<LittleEndian>(0).unwrap();
                    }
                    send_response(
                        &mut stream,
                        &ZKPacket::new(CMD_ACK_OK, session_id, req.reply_id, bytes),
                    );
                }
                CMD_GET_VERSION => {
                    send_response(
                        &mut stream,
                        &ZKPacket::new(
                            CMD_ACK_OK,
                            session_id,
                            req.reply_id,
                            b"Ver 6.60\0".to_vec(),
                        ),
                    );
                }
                CMD_GET_TIME => {
                    let t: u32 = ((25 * 12 * 31 + 5 * 31 + 14) * 86400) + (10 * 60 + 30) * 60;
                    let mut payload = Vec::new();
                    payload.write_u32::<LittleEndian>(t).unwrap();
                    send_response(
                        &mut stream,
                        &ZKPacket::new(CMD_ACK_OK, session_id, req.reply_id, payload),
                    );
                }
                CMD_EXIT => {
                    send_response(
                        &mut stream,
                        &ZKPacket::new(CMD_ACK_OK, session_id, req.reply_id, vec![]),
                    );
                    break;
                }
                _ => {
                    // Handle CMD_OPTIONS_RRQ (sync_timezone) and others
                    send_response(
                        &mut stream,
                        &ZKPacket::new(CMD_ACK_OK, session_id, req.reply_id, vec![]),
                    );
                }
            }
        }

        // Verify all expected empty-payload commands were seen
        assert!(
            empty_payload_cmds.contains(&CMD_CONNECT),
            "Should have seen CMD_CONNECT"
        );
        assert!(
            empty_payload_cmds.contains(&CMD_GET_FREE_SIZES),
            "Should have seen CMD_GET_FREE_SIZES"
        );
        assert!(
            empty_payload_cmds.contains(&CMD_GET_VERSION),
            "Should have seen CMD_GET_VERSION"
        );
        assert!(
            empty_payload_cmds.contains(&CMD_GET_TIME),
            "Should have seen CMD_GET_TIME"
        );
        assert!(
            empty_payload_cmds.contains(&CMD_EXIT),
            "Should have seen CMD_EXIT"
        );
    });

    let mut zk = ZK::new("127.0.0.1", port);
    zk.connect(ZKProtocol::TCP).unwrap();

    zk.read_sizes().unwrap();
    let fw = zk.get_firmware_version().unwrap();
    assert_eq!(fw, "Ver 6.60");
    let _time = zk.get_time().unwrap();
    zk.refresh_data().unwrap();
    zk.disconnect().unwrap();

    server.join().unwrap();
}

/// Verifies send_command works with stack arrays for fixed-size payloads.
/// Tests: unlock (4 bytes), set_time (4 bytes), delete_user (2 bytes).
#[test]
fn test_send_command_stack_array_payloads() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().unwrap().port();

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let session_id = 1234;

        // CMD_CONNECT
        let req = read_request(&mut stream);
        send_response(
            &mut stream,
            &ZKPacket::new(CMD_ACK_OK, session_id, req.reply_id, vec![]),
        );

        // CMD_UNLOCK — payload should be exactly 4 bytes
        let req = read_request(&mut stream);
        assert_eq!(req.command, CMD_UNLOCK);
        assert_eq!(
            req.payload.len(),
            4,
            "UNLOCK payload should be 4 bytes (stack array)"
        );
        let value = byteorder::LittleEndian::read_u32(&req.payload);
        assert_eq!(value, 50, "unlock(5) should send 5*10=50");
        send_response(
            &mut stream,
            &ZKPacket::new(CMD_ACK_OK, session_id, req.reply_id, vec![]),
        );

        // CMD_SET_TIME — payload should be exactly 4 bytes
        let req = read_request(&mut stream);
        assert_eq!(req.command, CMD_SET_TIME);
        assert_eq!(
            req.payload.len(),
            4,
            "SET_TIME payload should be 4 bytes (stack array)"
        );
        send_response(
            &mut stream,
            &ZKPacket::new(CMD_ACK_OK, session_id, req.reply_id, vec![]),
        );

        // CMD_DELETE_USER — payload should be exactly 2 bytes
        let req = read_request(&mut stream);
        assert_eq!(req.command, CMD_DELETE_USER);
        assert_eq!(
            req.payload.len(),
            2,
            "DELETE_USER payload should be 2 bytes (stack array)"
        );
        let uid = byteorder::LittleEndian::read_u16(&req.payload);
        assert_eq!(uid, 42);
        send_response(
            &mut stream,
            &ZKPacket::new(CMD_ACK_OK, session_id, req.reply_id, vec![]),
        );

        // CMD_REFRESHDATA (from refresh_data called inside delete_user)
        let req = read_request(&mut stream);
        assert_eq!(req.command, CMD_REFRESHDATA);
        send_response(
            &mut stream,
            &ZKPacket::new(CMD_ACK_OK, session_id, req.reply_id, vec![]),
        );

        // CMD_REG_EVENT — payload should be exactly 4 bytes
        let req = read_request(&mut stream);
        assert_eq!(req.command, CMD_REG_EVENT);
        assert_eq!(
            req.payload.len(),
            4,
            "REG_EVENT payload should be 4 bytes (stack array)"
        );
        let flags = byteorder::LittleEndian::read_u32(&req.payload);
        assert_eq!(flags, EF_ATTLOG);
        send_response(
            &mut stream,
            &ZKPacket::new(CMD_ACK_OK, session_id, req.reply_id, vec![]),
        );

        // CMD_EXIT
        let req = read_request(&mut stream);
        send_response(
            &mut stream,
            &ZKPacket::new(CMD_ACK_OK, session_id, req.reply_id, vec![]),
        );
    });

    let mut zk = ZK::new("127.0.0.1", port);
    zk.connect(ZKProtocol::TCP).unwrap();

    zk.unlock(5).unwrap();

    let now = chrono::Local::now();
    let now_fixed = chrono::DateTime::<chrono::FixedOffset>::from(now);
    zk.set_time(now_fixed).unwrap();

    zk.delete_user(42).unwrap();

    zk.reg_event(EF_ATTLOG).unwrap();

    zk.disconnect().unwrap();
    server.join().unwrap();
}

/// Verifies _CMD_PREPARE_BUFFER sends exactly 11 bytes as a stack array,
/// and _CMD_READ_BUFFER sends exactly 8 bytes as a stack array.
/// Uses a loop-based server to handle sync_timezone calls.
#[test]
fn test_buffer_commands_use_stack_arrays() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().unwrap().port();

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let session_id = 1234;
        let mut prepare_buf_payload_len: Option<usize> = None;
        let mut read_buf_payload_len: Option<usize> = None;

        loop {
            let req = read_request(&mut stream);

            match req.command {
                CMD_GET_FREE_SIZES => {
                    let mut bytes = Vec::new();
                    for i in 0..20 {
                        let val = match i {
                            4 => 1,    // users = 1
                            15 => 100, // users_cap
                            _ => 0,
                        };
                        bytes.write_i32::<LittleEndian>(val).unwrap();
                    }
                    send_response(
                        &mut stream,
                        &ZKPacket::new(CMD_ACK_OK, session_id, req.reply_id, bytes),
                    );
                }
                _CMD_PREPARE_BUFFER => {
                    prepare_buf_payload_len = Some(req.payload.len());
                    assert_eq!(req.payload[0], 1, "First byte should be ZK6/8 flag = 1");

                    let mut res_payload = vec![0u8; 5];
                    res_payload[0] = 1;
                    byteorder::LittleEndian::write_u32(&mut res_payload[1..5], 32);
                    send_response(
                        &mut stream,
                        &ZKPacket::new(CMD_PREPARE_DATA, session_id, req.reply_id, res_payload),
                    );
                }
                _CMD_READ_BUFFER => {
                    read_buf_payload_len = Some(req.payload.len());
                    let start = byteorder::LittleEndian::read_i32(&req.payload[0..4]);
                    assert_eq!(start, 0, "Start offset should be 0");

                    // Send user data
                    let mut data = Vec::new();
                    data.write_u32::<LittleEndian>(28).unwrap();
                    data.write_u16::<LittleEndian>(1).unwrap();
                    data.push(USER_DEFAULT);
                    data.extend_from_slice(b"pwd\0\0");
                    data.extend_from_slice(b"TestUsr\0");
                    data.write_u32::<LittleEndian>(0).unwrap();
                    data.push(0);
                    data.push(1);
                    data.write_u16::<LittleEndian>(0).unwrap();
                    data.write_u32::<LittleEndian>(1).unwrap();
                    send_response(
                        &mut stream,
                        &ZKPacket::new(CMD_DATA, session_id, req.reply_id, data),
                    );
                }
                CMD_EXIT => {
                    send_response(
                        &mut stream,
                        &ZKPacket::new(CMD_ACK_OK, session_id, req.reply_id, vec![]),
                    );
                    break;
                }
                _ => {
                    send_response(
                        &mut stream,
                        &ZKPacket::new(CMD_ACK_OK, session_id, req.reply_id, vec![]),
                    );
                }
            }
        }

        // Verify stack array sizes
        assert_eq!(
            prepare_buf_payload_len,
            Some(11),
            "_CMD_PREPARE_BUFFER payload should be 11 bytes (stack array)"
        );
        assert_eq!(
            read_buf_payload_len,
            Some(8),
            "_CMD_READ_BUFFER payload should be 8 bytes (stack array)"
        );
    });

    let mut zk = ZK::new("127.0.0.1", port);
    zk.connect(ZKProtocol::TCP).unwrap();

    let users = zk.get_users().unwrap();
    assert_eq!(users.len(), 1);
    assert_eq!(users[0].name, "TestUsr");

    zk.disconnect().unwrap();
    server.join().unwrap();
}

/// Verifies that encoding field is &'static str (no heap allocation).
#[test]
fn test_encoding_is_static_str() {
    let zk = ZK::new("192.168.1.1", 4370);
    assert_eq!(zk.encoding, "UTF-8");
    // If this compiles, encoding is &'static str (not String)
    let _: &'static str = zk.encoding;
}

/// Verifies pre-allocated Vecs: get_users/get_attendance produce correct results.
/// The pre-allocation (Vec::with_capacity) doesn't change behavior,
/// but this test confirms correctness is preserved.
#[test]
fn test_preallocated_results_correctness() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().unwrap().port();

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let session_id = 1234;
        let mut last_cmd = 0u16;

        loop {
            let req = read_request(&mut stream);

            match req.command {
                CMD_CONNECT => {
                    send_response(
                        &mut stream,
                        &ZKPacket::new(CMD_ACK_OK, session_id, req.reply_id, vec![]),
                    );
                }
                CMD_GET_FREE_SIZES => {
                    let mut bytes = Vec::new();
                    for i in 0..20 {
                        let val = match i {
                            4 => 3,      // users = 3
                            8 => 2,      // records = 2
                            15 => 1000,  // users_cap
                            16 => 10000, // rec_cap
                            _ => 0,
                        };
                        bytes.write_i32::<LittleEndian>(val).unwrap();
                    }
                    send_response(
                        &mut stream,
                        &ZKPacket::new(CMD_ACK_OK, session_id, req.reply_id, bytes),
                    );
                }
                _CMD_PREPARE_BUFFER => {
                    let cmd_in_buf = byteorder::LittleEndian::read_u16(&req.payload[1..3]);
                    last_cmd = cmd_in_buf;

                    let size: u32 = if cmd_in_buf == CMD_USERTEMP_RRQ {
                        4 + 28 * 3 // 3 users
                    } else {
                        4 + 8 * 2 // 2 attendance records
                    };
                    let mut res_payload = vec![0u8; 5];
                    res_payload[0] = 1;
                    byteorder::LittleEndian::write_u32(&mut res_payload[1..5], size);
                    send_response(
                        &mut stream,
                        &ZKPacket::new(CMD_PREPARE_DATA, session_id, req.reply_id, res_payload),
                    );
                }
                _CMD_READ_BUFFER => {
                    let mut data = Vec::new();

                    if last_cmd == CMD_USERTEMP_RRQ {
                        data.write_u32::<LittleEndian>(28 * 3).unwrap();
                        for i in 0..3u16 {
                            data.write_u16::<LittleEndian>(i + 1).unwrap(); // UID
                            data.push(USER_DEFAULT);
                            data.extend_from_slice(b"\0\0\0\0\0"); // Pass
                            let name = format!("User{}\0\0\0", i + 1);
                            data.extend_from_slice(&name.as_bytes()[..8]); // Name (8 bytes)
                            data.write_u32::<LittleEndian>(0).unwrap(); // Card
                            data.push(0); // Pad
                            data.push(1); // Group
                            data.write_u16::<LittleEndian>(0).unwrap(); // TZ
                            data.write_u32::<LittleEndian>((100 + i) as u32).unwrap();
                            // UserID
                        }
                    } else if last_cmd == CMD_ATTLOG_RRQ {
                        data.write_u32::<LittleEndian>(8 * 2).unwrap();
                        for i in 0..2u16 {
                            data.write_u16::<LittleEndian>(i + 1).unwrap(); // UID
                            data.push(1); // Status
                            data.write_u32::<LittleEndian>(839845230).unwrap(); // Time
                            data.push(0); // Punch
                        }
                    }
                    send_response(
                        &mut stream,
                        &ZKPacket::new(CMD_DATA, session_id, req.reply_id, data),
                    );
                }
                CMD_FREE_DATA => {
                    send_response(
                        &mut stream,
                        &ZKPacket::new(CMD_ACK_OK, session_id, req.reply_id, vec![]),
                    );
                }
                CMD_EXIT => {
                    send_response(
                        &mut stream,
                        &ZKPacket::new(CMD_ACK_OK, session_id, req.reply_id, vec![]),
                    );
                    break;
                }
                _ => {
                    send_response(
                        &mut stream,
                        &ZKPacket::new(CMD_ACK_OK, session_id, req.reply_id, vec![]),
                    );
                }
            }
        }
    });

    let mut zk = ZK::new("127.0.0.1", port);
    zk.connect(ZKProtocol::TCP).unwrap();

    // Verify users (should have capacity pre-allocated for 3)
    let users = zk.get_users().unwrap();
    assert_eq!(users.len(), 3, "Should return exactly 3 users");
    assert_eq!(users[0].name, "User1");
    assert_eq!(users[1].name, "User2");
    assert_eq!(users[2].name, "User3");
    assert_eq!(users[0].user_id, "100");
    assert_eq!(users[2].user_id, "102");

    // Verify attendance (should have capacity pre-allocated for 2)
    let logs = zk.get_attendance().unwrap();
    assert_eq!(logs.len(), 2, "Should return exactly 2 attendance records");

    zk.disconnect().unwrap();
    server.join().unwrap();
}
