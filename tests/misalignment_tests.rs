use byteorder::{ByteOrder, LittleEndian, WriteBytesExt};
use chrono::Datelike;
use rustzk::constants::*;
use rustzk::protocol::{TCPWrapper, ZKPacket};
use rustzk::{ZKProtocol, ZK};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

#[test]
fn test_request_response_misalignment_tcp() {
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

        // 2. Handle CMD_GET_TIME (Client will trigger automated TZAdj sync first)
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

        // 3. Handle the actual CMD_GET_TIME
        stream.read_exact(&mut header).unwrap();
        let (length, _) = TCPWrapper::decode_header(&header).unwrap();
        let mut body = vec![0u8; length];
        stream.read_exact(&mut body).unwrap();
        let packet = ZKPacket::from_bytes_owned(body).unwrap();
        let expected_reply_id = packet.reply_id;

        // MOCK MISALIGNMENT: Send a STALE packet first with an old reply_id
        let stale_res = ZKPacket::new(
            CMD_ACK_OK,
            session_id,
            expected_reply_id.wrapping_sub(1),
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

#[test]
fn test_receive_chunk_misalignment_tcp() {
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

        // 2. Handle CMD_GET_FREE_SIZES (called by read_sizes)
        stream.read_exact(&mut header).unwrap();
        let (length, _) = TCPWrapper::decode_header(&header).unwrap();
        let mut body = vec![0u8; length];
        stream.read_exact(&mut body).unwrap();
        let packet = ZKPacket::from_bytes_owned(body).unwrap();
        assert_eq!(packet.command, CMD_GET_FREE_SIZES);

        let mut sizes_payload = vec![0u8; 92];
        // self.users is at fields[4] which is offset 16..20 (4*4)
        LittleEndian::write_i32(&mut sizes_payload[16..20], 1);
        let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, sizes_payload);
        stream
            .write_all(&TCPWrapper::wrap(&res.to_bytes()))
            .unwrap();

        // 2.5 Handle TZAdj Sync (from read_sizes)
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

        // 3. Handle CMD_USERTEMP_RRQ (from get_users -> read_with_buffer)
        stream.read_exact(&mut header).unwrap();
        let (length, _) = TCPWrapper::decode_header(&header).unwrap();
        let mut body = vec![0u8; length];
        stream.read_exact(&mut body).unwrap();
        let packet = ZKPacket::from_bytes_owned(body).unwrap();
        assert_eq!(packet.command, _CMD_PREPARE_BUFFER);

        // Send CMD_PREPARE_DATA with size 28 (one user)
        let mut prepare_payload = Vec::new();
        prepare_payload.write_u32::<LittleEndian>(28).unwrap();
        let res = ZKPacket::new(
            CMD_PREPARE_DATA,
            session_id,
            packet.reply_id,
            prepare_payload,
        );
        stream
            .write_all(&TCPWrapper::wrap(&res.to_bytes()))
            .unwrap();

        // 4. Handle _CMD_READ_BUFFER (from read_chunk)
        stream.read_exact(&mut header).unwrap();
        let (length, _) = TCPWrapper::decode_header(&header).unwrap();
        let mut body = vec![0u8; length];
        stream.read_exact(&mut body).unwrap();
        let packet = ZKPacket::from_bytes_owned(body).unwrap();
        assert_eq!(packet.command, _CMD_READ_BUFFER);

        // Send CMD_PREPARE_DATA for the chunk
        let mut chunk_prepare_payload = Vec::new();
        chunk_prepare_payload.write_u32::<LittleEndian>(32).unwrap(); // 4 bytes size + 28 bytes user
        let res = ZKPacket::new(
            CMD_PREPARE_DATA,
            session_id,
            packet.reply_id,
            chunk_prepare_payload,
        );
        stream
            .write_all(&TCPWrapper::wrap(&res.to_bytes()))
            .unwrap();

        // 5. Client enters receive_chunk loop.
        // MOCK MISALIGNMENT: Send a STALE CMD_DATA packet first
        let mut stale_data = Vec::new();
        stale_data.write_u32::<LittleEndian>(28).unwrap(); // size of data following
        stale_data.extend_from_slice(&[0xEE; 28]);
        let stale_res = ZKPacket::new(CMD_DATA, session_id, packet.reply_id - 1, stale_data);
        stream
            .write_all(&TCPWrapper::wrap(&stale_res.to_bytes()))
            .unwrap();
        stream.flush().unwrap();

        // Then send the CORRECT CMD_DATA packet
        // User struct: uid(2), priv(1), pass(5), name(8), card(4), pad(1), group(1), tz(2), user_id(4) = 28 bytes
        let mut correct_data = Vec::new();
        correct_data.write_u32::<LittleEndian>(28).unwrap(); // size
        correct_data.write_u16::<LittleEndian>(1).unwrap(); // uid
        correct_data.write_u8(0).unwrap(); // priv
        correct_data.extend_from_slice(b"12345"); // pass
        correct_data.extend_from_slice(b"TestUser"); // name
        correct_data.write_u32::<LittleEndian>(0).unwrap(); // card
        correct_data.write_u8(0).unwrap(); // pad
        correct_data.write_u8(1).unwrap(); // group
        correct_data.write_u16::<LittleEndian>(0).unwrap(); // tz
        correct_data.write_u32::<LittleEndian>(100).unwrap(); // user_id

        let correct_res = ZKPacket::new(CMD_DATA, session_id, packet.reply_id, correct_data);
        stream
            .write_all(&TCPWrapper::wrap(&correct_res.to_bytes()))
            .unwrap();
        stream.flush().unwrap();

        // 6. Handle CMD_FREE_DATA (sent after read_with_buffer finishes)
        stream.read_exact(&mut header).unwrap();
        let (length, _) = TCPWrapper::decode_header(&header).unwrap();
        let mut body = vec![0u8; length];
        stream.read_exact(&mut body).unwrap();
        let packet = ZKPacket::from_bytes_owned(body).unwrap();
        assert_eq!(packet.command, CMD_FREE_DATA);

        let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, vec![]);
        stream
            .write_all(&TCPWrapper::wrap(&res.to_bytes()))
            .unwrap();
        stream.flush().unwrap();
    });

    let mut zk = ZK::new("127.0.0.1", port);
    zk.connect(ZKProtocol::TCP).unwrap();

    let users = zk.get_users().unwrap();
    assert_eq!(users.len(), 1);
    assert_eq!(users[0].name, "TestUser");
    assert_eq!(users[0].user_id, "100");

    server_handle.join().unwrap();
}
