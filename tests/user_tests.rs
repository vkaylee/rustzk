use rustzk::constants::*;
use rustzk::protocol::{TCPWrapper, ZKPacket};
use rustzk::models::User;
use rustzk::{ZKProtocol, ZK};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

#[test]
fn test_set_user_mock() {
    let _ = env_logger::builder().is_test(true).try_init();
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind");
    let addr = listener.local_addr().unwrap();
    let port = addr.port();

    let server_handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let session_id = 8888;

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
                    stream.write_all(&TCPWrapper::wrap(&res.to_bytes())).unwrap();
                }
                CMD_USER_WRQ => {
                    // 28-byte format check (default for new ZK)
                    assert_eq!(packet.payload.len(), 28);
                    let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, vec![]);
                    stream.write_all(&TCPWrapper::wrap(&res.to_bytes())).unwrap();
                }
                CMD_REFRESHDATA => {
                    let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, vec![]);
                    stream.write_all(&TCPWrapper::wrap(&res.to_bytes())).unwrap();
                }
                CMD_EXIT => {
                    let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, vec![]);
                    stream.write_all(&TCPWrapper::wrap(&res.to_bytes())).unwrap();
                    break;
                }
                _ => {
                    let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, vec![]);
                    stream.write_all(&TCPWrapper::wrap(&res.to_bytes())).unwrap();
                }
            }
        }
    });

    let mut zk = ZK::new("127.0.0.1", port);
    zk.connect(ZKProtocol::TCP).unwrap();

    let user = User {
        uid: 1,
        name: "Test User".to_string(),
        privilege: USER_ADMIN,
        password: "123".to_string(),
        group_id: "1".to_string(),
        user_id: "101".to_string(),
        card: 0,
    };

    let result = zk.set_user(&user);
    assert!(result.is_ok());

    zk.disconnect().unwrap();
    server_handle.join().unwrap();
}

#[test]
fn test_delete_user_mock() {
    let _ = env_logger::builder().is_test(true).try_init();
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind");
    let addr = listener.local_addr().unwrap();
    let port = addr.port();

    let server_handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let session_id = 9999;

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
                    stream.write_all(&TCPWrapper::wrap(&res.to_bytes())).unwrap();
                }
                CMD_DELETE_USER => {
                    assert_eq!(packet.payload.len(), 2);
                    let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, vec![]);
                    stream.write_all(&TCPWrapper::wrap(&res.to_bytes())).unwrap();
                }
                CMD_REFRESHDATA => {
                    let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, vec![]);
                    stream.write_all(&TCPWrapper::wrap(&res.to_bytes())).unwrap();
                }
                CMD_EXIT => {
                    let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, vec![]);
                    stream.write_all(&TCPWrapper::wrap(&res.to_bytes())).unwrap();
                    break;
                }
                _ => {
                    let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, vec![]);
                    stream.write_all(&TCPWrapper::wrap(&res.to_bytes())).unwrap();
                }
            }
        }
    });

    let mut zk = ZK::new("127.0.0.1", port);
    zk.connect(ZKProtocol::TCP).unwrap();

    let result = zk.delete_user(1);
    assert!(result.is_ok());

    zk.disconnect().unwrap();
    server_handle.join().unwrap();
}
