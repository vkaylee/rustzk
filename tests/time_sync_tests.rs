use byteorder::{LittleEndian, WriteBytesExt};
use rustzk::constants::*;
use rustzk::protocol::{TCPWrapper, ZKPacket};
use rustzk::{ZKProtocol, ZK};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

#[test]
fn test_lazy_timezone_sync_on_get_time() {
    let _ = env_logger::builder().is_test(true).try_init();
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
        let packet = ZKPacket::from_bytes_owned(body).unwrap();
        assert_eq!(packet.command, CMD_CONNECT);

        let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, vec![]);
        stream
            .write_all(&TCPWrapper::wrap(&res.to_bytes()))
            .unwrap();

        // 2. Handle Lazy TZAdj Sync (triggered by get_time)
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

        // 3. Handle CMD_GET_TIME
        stream.read_exact(&mut header).unwrap();
        let (length, _) = TCPWrapper::decode_header(&header).unwrap();
        let mut body = vec![0u8; length];
        stream.read_exact(&mut body).unwrap();
        let packet = ZKPacket::from_bytes_owned(body).unwrap();
        assert_eq!(packet.command, CMD_GET_TIME);

        // Send a known time (e.g., 2026-02-20 10:00:00)
        let t: u32 = 839845230; // Encoded time for known date
        let mut payload = Vec::new();
        payload.write_u32::<LittleEndian>(t).unwrap();

        let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, payload);
        stream
            .write_all(&TCPWrapper::wrap(&res.to_bytes()))
            .unwrap();
    });

    let mut zk = ZK::new("127.0.0.1", port);

    // 1. Connect (should NOT sync timezone yet)
    zk.connect(ZKProtocol::TCP).unwrap();
    assert!(!zk.timezone_synced);
    assert_eq!(zk.timezone_offset, 0);

    // 2. Call get_time (should trigger lazy sync)
    let time = zk.get_time().unwrap();

    // 3. Verify sync happened
    assert!(zk.timezone_synced);
    assert_eq!(zk.timezone_offset, 7 * 60); // 420 minutes
    assert_eq!(time.timezone().local_minus_utc(), 7 * 3600);

    server_handle.join().unwrap();
}

#[test]
fn test_lazy_sync_happens_only_once() {
    let _ = env_logger::builder().is_test(true).try_init();
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind mock server");
    let addr = listener.local_addr().unwrap();
    let port = addr.port();

    let server_handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("Failed to accept connection");
        let session_id = 5678;

        // 1. Connect
        let mut header = [0u8; 8];
        stream.read_exact(&mut header).unwrap();
        let (length, _) = TCPWrapper::decode_header(&header).unwrap();
        let mut body = vec![0u8; length];
        stream.read_exact(&mut body).unwrap();
        let pkt1 = ZKPacket::from_bytes_owned(body).unwrap();
        let res = ZKPacket::new(CMD_ACK_OK, session_id, pkt1.reply_id, vec![]);
        stream
            .write_all(&TCPWrapper::wrap(&res.to_bytes()))
            .unwrap();

        // 2. Lazy Sync (First call)
        stream.read_exact(&mut header).unwrap();
        let (length, _) = TCPWrapper::decode_header(&header).unwrap();
        let mut body = vec![0u8; length];
        stream.read_exact(&mut body).unwrap();
        let packet = ZKPacket::from_bytes_owned(body).unwrap();
        assert_eq!(packet.command, CMD_OPTIONS_RRQ, "Should ask for TZAdj");

        let res = ZKPacket::new(
            CMD_ACK_OK,
            session_id,
            packet.reply_id,
            b"TZAdj=8\0".to_vec(),
        );
        stream
            .write_all(&TCPWrapper::wrap(&res.to_bytes()))
            .unwrap();

        // 3. CMD_GET_TIME (First call)
        stream.read_exact(&mut header).unwrap();
        let (length, _) = TCPWrapper::decode_header(&header).unwrap();
        let mut body = vec![0u8; length];
        stream.read_exact(&mut body).unwrap();
        let pkt3 = ZKPacket::from_bytes_owned(body).unwrap();
        // Respond with OK
        let res = ZKPacket::new(
            CMD_ACK_OK,
            session_id,
            pkt3.reply_id,
            839845230u32.to_le_bytes().to_vec(),
        );
        stream
            .write_all(&TCPWrapper::wrap(&res.to_bytes()))
            .unwrap();

        // 4. CMD_GET_TIME (Second call) - Should NOT see another CMD_OPTIONS_RRQ
        stream.read_exact(&mut header).unwrap();
        let (length, _) = TCPWrapper::decode_header(&header).unwrap();
        let mut body = vec![0u8; length];
        stream.read_exact(&mut body).unwrap();
        let packet = ZKPacket::from_bytes_owned(body).unwrap();
        assert_eq!(
            packet.command, CMD_GET_TIME,
            "Should immediately call GET_TIME, no re-sync"
        );

        // Respond with OK
        let res = ZKPacket::new(
            CMD_ACK_OK,
            session_id,
            packet.reply_id,
            839845230u32.to_le_bytes().to_vec(),
        );
        stream
            .write_all(&TCPWrapper::wrap(&res.to_bytes()))
            .unwrap();
    });

    let mut zk = ZK::new("127.0.0.1", port);
    zk.connect(ZKProtocol::TCP).unwrap();

    // First call: Triggers sync
    zk.get_time().unwrap();
    assert!(zk.timezone_synced);
    assert_eq!(zk.timezone_offset, 8 * 60);

    // Second call: Should reuse cached offset
    zk.get_time().unwrap();
    assert!(zk.timezone_synced);

    server_handle.join().unwrap();
}
