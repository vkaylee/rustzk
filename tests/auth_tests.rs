use rustzk::constants::*;
use rustzk::protocol::{TCPWrapper, ZKPacket};
use rustzk::{ZKProtocol, ZK};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

#[test]
fn test_auth_handshake_mock() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind mock server");
    let addr = listener.local_addr().unwrap();
    let port = addr.port();

    let server_handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("Failed to accept connection");
        let session_id = 9876;
        let mut authenticated = false;

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
                    // Respond with UNAUTH to trigger client auth flow
                    let res = ZKPacket::new(CMD_ACK_UNAUTH, session_id, packet.reply_id, vec![]);
                    stream
                        .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                        .unwrap();
                }
                CMD_AUTH => {
                    // Check if payload has 4 bytes (commkey)
                    if packet.payload.len() == 4 {
                        authenticated = true;
                        let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, vec![]);
                        stream
                            .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                            .unwrap();
                    } else {
                        let res = ZKPacket::new(CMD_ACK_ERROR, session_id, packet.reply_id, vec![]);
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
                    if authenticated {
                        let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, vec![]);
                        stream
                            .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                            .unwrap();
                    } else {
                        let res =
                            ZKPacket::new(CMD_ACK_UNAUTH, session_id, packet.reply_id, vec![]);
                        stream
                            .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                            .unwrap();
                    }
                }
            }
        }
    });

    let mut zk = ZK::new("127.0.0.1", port);
    zk.set_password(0); // Using default pass 0

    // connect() should handle CMD_ACK_UNAUTH and send CMD_AUTH automatically
    let res = zk.connect(ZKProtocol::TCP);
    assert!(res.is_ok(), "Connection with auth should succeed");
    assert!(zk.is_connected, "ZK should be marked as connected");

    zk.disconnect().unwrap();
    server_handle.join().unwrap();
}

#[test]
fn test_connect_with_password_mock() {
    let _ = env_logger::builder().is_test(true).try_init();
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind");
    let addr = listener.local_addr().unwrap();
    let port = addr.port();

    let server_handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let session_id = 5555;

        loop {
            let mut header = [0u8; 8];
            if stream.read_exact(&mut header).is_err() { break; }
            let (length, _) = TCPWrapper::decode_header(&header).unwrap();
            let mut body = vec![0u8; length];
            stream.read_exact(&mut body).unwrap();
            let packet = ZKPacket::from_bytes_owned(body).unwrap();

            match packet.command {
                CMD_CONNECT => {
                    // Force UNAUTH to trigger password flow
                    let res = ZKPacket::new(CMD_ACK_UNAUTH, session_id, packet.reply_id, vec![]);
                    stream.write_all(&TCPWrapper::wrap(&res.to_bytes())).unwrap();
                }
                CMD_AUTH => {
                    // Check if password payload exists (4 bytes)
                    assert_eq!(packet.payload.len(), 4);
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
    zk.set_password(123456); // Set test password
    
    let result = zk.connect(ZKProtocol::TCP);
    assert!(result.is_ok());
    assert!(zk.is_connected);

    zk.disconnect().unwrap();
    server_handle.join().unwrap();
}
