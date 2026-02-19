use byteorder::{LittleEndian, WriteBytesExt};
use chrono::Datelike;
use rustzk::constants::*;
use rustzk::protocol::{TCPWrapper, ZKPacket};
use rustzk::{ZKProtocol, ZK};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

#[test]
fn test_request_response_misalignment_tcp() {
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
        assert_eq!(packet.command, CMD_CONNECT);

        let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, vec![]);
        stream
            .write_all(&TCPWrapper::wrap(&res.to_bytes()))
            .unwrap();

        // 2. Handle CMD_GET_TIME - Client expects reply_id = 0 (since it wraps or starts at USHRT_MAX-1)
        // Wait, ZK::new sets reply_id = USHRT_MAX - 1.
        // connect increments it to 0.
        // get_time increments it to 1.

        stream.read_exact(&mut header).unwrap();
        let (length, _) = TCPWrapper::decode_header(&header).unwrap();
        let mut body = vec![0u8; length];
        stream.read_exact(&mut body).unwrap();
        let packet = ZKPacket::from_bytes(&body).unwrap();
        let expected_reply_id = packet.reply_id;

        // MOCK MISALIGNMENT: Send a STALE packet first with an old reply_id
        let stale_res = ZKPacket::new(
            CMD_ACK_OK,
            session_id,
            expected_reply_id - 1,
            vec![b'S', b'T', b'A', b'L', b'E'],
        );
        stream
            .write_all(&TCPWrapper::wrap(&stale_res.to_bytes()))
            .unwrap();
        stream.flush().unwrap();

        // Then send the CORRECT packet
        let mut time_payload = Vec::new();
        time_payload.write_u32::<LittleEndian>(839845230).unwrap();
        let correct_res = ZKPacket::new(CMD_ACK_OK, session_id, expected_reply_id, time_payload);
        stream
            .write_all(&TCPWrapper::wrap(&correct_res.to_bytes()))
            .unwrap();
        stream.flush().unwrap();
    });

    let mut zk = ZK::new("127.0.0.1", port);
    zk.connect(ZKProtocol::TCP).unwrap();

    // This call should skip the "STALE" packet and get the correct time
    let time = zk.get_time().unwrap();
    assert_eq!(time.year(), 2026); // Based on 839845230 calculation if I'm not wrong, but focus is on it NOT failing

    server_handle.join().unwrap();
}
