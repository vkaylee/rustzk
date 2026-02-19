use byteorder::{ByteOrder, LittleEndian, WriteBytesExt};
use rustzk::constants::*;
use rustzk::protocol::{TCPWrapper, ZKPacket};
use rustzk::{ZKProtocol, ZK};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;
use std::time::Duration;

#[test]
fn test_read_chunk_waits_for_data_after_ack() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind mock server");
    let addr = listener.local_addr().unwrap();
    let port = addr.port();

    let server_handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("Failed to accept connection");
        let session_id = 9999;

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
                    let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, vec![]);
                    stream
                        .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                        .unwrap();
                }
                CMD_GET_FREE_SIZES => {
                    let mut bytes = Vec::new();
                    for i in 0..20 {
                        // idx 4=users, 8=records. Set 1 user, 1 record.
                        let val = if i == 4 || i == 8 { 1 } else { 0 };
                        bytes.write_i32::<LittleEndian>(val).unwrap();
                    }
                    let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, bytes);
                    stream
                        .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                        .unwrap();
                }
                _CMD_PREPARE_BUFFER => {
                    // Always say size is 4 (size header) + 28 (User struct) = 32
                    let size = 4 + 28;
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
                    // HERE IS THE TEST:
                    // 1. Send ACK immediately.
                    // 2. Send DATA immediately after.
                    // The client MUST NOT send another request.

                    // 1. Send ACK
                    let ack = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, vec![]);
                    stream
                        .write_all(&TCPWrapper::wrap(&ack.to_bytes()))
                        .unwrap();
                    stream.flush().unwrap();

                    // Sleep tiny bit to ensure client processes ACK and enters "wait for data" state
                    // logic: if it returns from send_command with ACK, it calls read_response_safe.
                    thread::sleep(Duration::from_millis(10));

                    // 2. Send DATA
                    let mut data = Vec::new();
                    data.write_u32::<LittleEndian>(28).unwrap(); // Size prefix
                    data.write_u16::<LittleEndian>(1).unwrap(); // UID
                    data.push(USER_DEFAULT); // Priv
                    data.extend_from_slice(b"pwd\0\0"); // Pass
                    data.extend_from_slice(b"Test\0\0\0\0"); // Name
                    data.write_u32::<LittleEndian>(0).unwrap(); // Card
                    data.push(0);
                    data.push(1);
                    data.write_u16::<LittleEndian>(0).unwrap();
                    data.write_u32::<LittleEndian>(101).unwrap(); // UserID

                    let res = ZKPacket::new(CMD_DATA, session_id, packet.reply_id, data);
                    stream
                        .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                        .unwrap();
                }
                CMD_FREE_DATA => {
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
    zk.connect(ZKProtocol::TCP).expect("Connect failed");

    // This should trigger the READ_BUFFER flow
    let users = zk.get_users().expect("get_users failed");

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].name, "Test");

    zk.disconnect().unwrap();
    server_handle.join().unwrap();
}
