use byteorder::{ByteOrder, LittleEndian, WriteBytesExt};
use rustzk::constants::*;
use rustzk::protocol::{TCPWrapper, ZKPacket};
use rustzk::{ZKProtocol, ZK};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

#[test]
fn test_get_option_and_timezone_mock() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind mock server");
    let addr = listener.local_addr().unwrap();
    let port = addr.port();

    let server_handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("Failed to accept connection");
        let mut session_id = 0;

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
                    session_id = 5566;
                    let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, vec![]);
                    stream
                        .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                        .unwrap();
                }
                CMD_OPTIONS_RRQ => {
                    let key = String::from_utf8_lossy(&packet.payload).trim_matches('\0').to_string();
                    let response_str = match key.as_str() {
                        "TZAdj" => "TZAdj=7\0",
                        "~SerialNumber" => "~SerialNumber=ABC12345\0",
                        _ => "Unknown=0\0",
                    };
                    
                    let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, response_str.as_bytes().to_vec());
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
                    let res = ZKPacket::new(CMD_ACK_UNKNOWN, session_id, packet.reply_id, vec![]);
                    stream.write_all(&TCPWrapper::wrap(&res.to_bytes())).unwrap();
                }
            }
        }
    });

    let mut zk = ZK::new("127.0.0.1", port);
    zk.connect(ZKProtocol::Auto).unwrap();

    // Test get_option_value
    let sn = zk.get_option_value("~SerialNumber").unwrap();
    assert_eq!(sn, "ABC12345");

    // Test get_timezone
    let tz = zk.get_timezone().unwrap();
    assert_eq!(tz, 7);

    zk.disconnect().unwrap();
    server_handle.join().unwrap();
}

#[test]
fn test_attendance_heuristic_40bytes_mock() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind mock server");
    let addr = listener.local_addr().unwrap();
    let port = addr.port();

    let server_handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("Failed to accept connection");
        let mut session_id = 0;

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
                    session_id = 7788;
                    let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, vec![]);
                    stream.write_all(&TCPWrapper::wrap(&res.to_bytes())).unwrap();
                }
                CMD_GET_FREE_SIZES => {
                    let mut bytes = Vec::new();
                    for i in 0..20 {
                        let val = if i == 8 { 1 } else { 0 }; // Report 1 record
                        bytes.write_i32::<LittleEndian>(val).unwrap();
                    }
                    let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, bytes);
                    stream.write_all(&TCPWrapper::wrap(&res.to_bytes())).unwrap();
                }
                _CMD_PREPARE_BUFFER => {
                    let mut res_payload = vec![0u8; 5];
                    res_payload[0] = 1;
                    // Total size = 4 (prefix) + 40 (one 40-byte record) = 44
                    LittleEndian::write_u32(&mut res_payload[1..5], 44);

                    let res = ZKPacket::new(CMD_PREPARE_DATA, session_id, packet.reply_id, res_payload);
                    stream.write_all(&TCPWrapper::wrap(&res.to_bytes())).unwrap();
                }
                _CMD_READ_BUFFER => {
                    let mut data = Vec::new();
                    data.write_u32::<LittleEndian>(40).unwrap(); // Size prefix
                    
                    // 40-byte record
                    data.write_u16::<LittleEndian>(10).unwrap(); // UID
                    let mut user_id_bytes = [0u8; 24];
                    user_id_bytes[0..3].copy_from_slice(b"101");
                    data.write_all(&user_id_bytes).unwrap();
                    data.push(1); // Status
                    data.write_u32::<LittleEndian>(839845230).unwrap(); // Time
                    data.push(0); // Punch
                    data.write_all(&[0u8; 8]).unwrap(); // Padding to reach 40 bytes

                    let res = ZKPacket::new(CMD_DATA, session_id, packet.reply_id, data);
                    stream.write_all(&TCPWrapper::wrap(&res.to_bytes())).unwrap();
                }
                CMD_EXIT => { break; }
                _ => {
                    let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, vec![]);
                    stream.write_all(&TCPWrapper::wrap(&res.to_bytes())).unwrap();
                }
            }
        }
    });

    let mut zk = ZK::new("127.0.0.1", port);
    zk.connect(ZKProtocol::TCP).unwrap();

    let logs = zk.get_attendance().unwrap();
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].user_id, "101");
    assert_eq!(logs[0].uid, 10);

    zk.disconnect().unwrap();
    server_handle.join().unwrap();
}
