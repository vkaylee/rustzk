use byteorder::{ByteOrder, LittleEndian, WriteBytesExt};
use rustzk::constants::*;
use rustzk::protocol::{TCPWrapper, ZKPacket};
use rustzk::ZK;
use std::io::{Read, Write};
use std::net::TcpListener;
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
    zk.connect(true).unwrap();

    zk.read_sizes().unwrap();
    assert_eq!(zk.users, 1, "Users should be 1");
    assert_eq!(zk.records, 1, "Records should be 1");

    let users = zk.get_users().unwrap();
    assert_eq!(users.len(), 1);
    assert_eq!(users[0].name, "Admin");

    let logs = zk.get_attendance().unwrap();
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].user_id, "101");

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
    let result = zk.connect(false);

    assert!(result.is_ok());
    assert!(zk.is_connected);

    server_handle.join().unwrap();
}
