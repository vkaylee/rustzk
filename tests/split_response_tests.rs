use byteorder::{ByteOrder, LittleEndian};
use rustzk::constants::*;
use rustzk::protocol::{TCPWrapper, ZKPacket};
use rustzk::{ZKProtocol, ZK};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

#[test]
fn test_read_sizes_split_response_tcp() {
    let _ = env_logger::builder().is_test(true).try_init();
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind mock server");
    let addr = listener.local_addr().unwrap();
    let port = addr.port();

    let server_handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("Failed to accept connection");
        let session_id = 5678;

        // 1. Handle Connect
        let mut header = [0u8; 8];
        stream.read_exact(&mut header).unwrap();
        let (length, _) = TCPWrapper::decode_header(&header).unwrap();
        let mut body = vec![0u8; length];
        stream.read_exact(&mut body).unwrap();
        let packet = ZKPacket::from_bytes_owned(body).unwrap();
        assert_eq!(packet.command, CMD_CONNECT);

        let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, vec![]);
        stream
            .write_all(&TCPWrapper::wrap(&res.to_bytes()))
            .unwrap();
        stream.flush().unwrap();

        // 2. Handle CMD_GET_FREE_SIZES
        stream.read_exact(&mut header).unwrap();
        let (length, _) = TCPWrapper::decode_header(&header).unwrap();
        let mut body = vec![0u8; length];
        stream.read_exact(&mut body).unwrap();
        let packet = ZKPacket::from_bytes_owned(body).unwrap();
        assert_eq!(packet.command, CMD_GET_FREE_SIZES);

        // SYIMULATE BUG/SPLIT:
        // First send an empty ACK_OK
        let ack_ok = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, vec![]);
        stream
            .write_all(&TCPWrapper::wrap(&ack_ok.to_bytes()))
            .unwrap();
        stream.flush().unwrap();

        // Then send the actual DATA packet (CMD_ACK_DATA or another ACK_OK with payload)
        // Some devices send another ACK_OK with payload, or ACK_DATA.
        // Our fix handles both if it's the second packet read.
        let mut sizes_payload = vec![0u8; 92];
        // Set user count to 123 (fields[4] is offset 16..20)
        LittleEndian::write_i32(&mut sizes_payload[16..20], 123);
        // Set record count to 456 (fields[8] is offset 32..36)
        LittleEndian::write_i32(&mut sizes_payload[32..36], 456);

        let data_res = ZKPacket::new(CMD_ACK_DATA, session_id, packet.reply_id, sizes_payload);
        stream
            .write_all(&TCPWrapper::wrap(&data_res.to_bytes()))
            .unwrap();
        stream.flush().unwrap();

        // 2.5 Handle TZAdj Sync (Automated from read_sizes)
        stream.read_exact(&mut header).unwrap();
        let (length, _) = TCPWrapper::decode_header(&header).unwrap();
        let mut body = vec![0u8; length];
        stream.read_exact(&mut body).unwrap();
        let packet = ZKPacket::from_bytes_owned(body).unwrap();
        assert_eq!(packet.command, CMD_OPTIONS_RRQ);
        let res = ZKPacket::new(
            CMD_ACK_OK,
            session_id,
            packet.reply_id,
            b"TZAdj=7\0".to_vec(),
        );
        stream
            .write_all(&TCPWrapper::wrap(&res.to_bytes()))
            .unwrap();
        stream.flush().unwrap();
    });

    let mut zk = ZK::new("127.0.0.1", port);
    zk.connect(ZKProtocol::TCP).unwrap();

    // Trigger read_sizes
    zk.read_sizes()
        .expect("read_sizes should handle split response");

    // Verify fields were populated from the SECOND packet
    assert_eq!(zk.users(), 123);
    assert_eq!(zk.records(), 456);

    server_handle.join().unwrap();
}
