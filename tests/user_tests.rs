use byteorder::{ByteOrder, LittleEndian, WriteBytesExt};
use rustzk::constants::*;
use rustzk::models::User;
use rustzk::protocol::{TCPWrapper, ZKPacket};
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
                    stream
                        .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                        .unwrap();
                }
                CMD_GET_FREE_SIZES => {
                    let mut bytes = Vec::new();
                    for _ in 0..20 {
                        bytes.write_i32::<LittleEndian>(0).unwrap();
                    }
                    let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, bytes);
                    stream
                        .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                        .unwrap();
                }
                _CMD_PREPARE_BUFFER => {
                    let res = ZKPacket::new(
                        CMD_PREPARE_DATA,
                        session_id,
                        packet.reply_id,
                        vec![1, 4, 0, 0, 0],
                    );
                    stream
                        .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                        .unwrap();
                }
                _CMD_READ_BUFFER => {
                    let res =
                        ZKPacket::new(CMD_DATA, session_id, packet.reply_id, vec![0, 0, 0, 0]);
                    stream
                        .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                        .unwrap();
                }
                CMD_USER_WRQ => {
                    let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, vec![]);
                    stream
                        .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                        .unwrap();
                }
                CMD_REFRESHDATA => {
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
fn test_set_user_conflict_mock() {
    let _ = env_logger::builder().is_test(true).try_init();
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind");
    let addr = listener.local_addr().unwrap();
    let port = addr.port();

    let server_handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let session_id = 7777;

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
                    stream
                        .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                        .unwrap();
                }
                CMD_GET_FREE_SIZES => {
                    let mut bytes = vec![0u8; 80];
                    <LittleEndian as ByteOrder>::write_i32(&mut bytes[16..20], 1); // 1 user
                    let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, bytes);
                    stream
                        .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                        .unwrap();
                }
                _CMD_PREPARE_BUFFER => {
                    let mut res_payload = vec![0u8; 5];
                    res_payload[0] = 1;
                    <LittleEndian as ByteOrder>::write_u32(&mut res_payload[1..5], 32); // 4 + 28
                    let res =
                        ZKPacket::new(CMD_PREPARE_DATA, session_id, packet.reply_id, res_payload);
                    stream
                        .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                        .unwrap();
                }
                _CMD_READ_BUFFER => {
                    let mut data = Vec::new();
                    data.write_i32::<LittleEndian>(28).unwrap();
                    data.write_u16::<LittleEndian>(10).unwrap(); // UID 10
                    data.write_u8(0).unwrap();
                    data.write_all(&[0u8; 5]).unwrap();
                    data.write_all(&[0u8; 8]).unwrap();
                    data.write_u32::<LittleEndian>(0).unwrap();
                    data.write_u8(0).unwrap();
                    data.write_u8(1).unwrap();
                    data.write_u16::<LittleEndian>(0).unwrap();
                    data.write_i32::<LittleEndian>(101).unwrap(); // ID "101"

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
    zk.connect(ZKProtocol::TCP).unwrap();

    // Conflict case: Adding ID "101" at UID 11, but it already exists at UID 10
    let user = User {
        uid: 11,
        user_id: "101".to_string(),
        name: "Cloned User".to_string(),
        privilege: USER_DEFAULT,
        password: "".to_string(),
        group_id: "1".to_string(),
        card: 0,
    };

    let result = zk.set_user(&user);
    assert!(result.is_err());
    assert!(format!("{}", result.unwrap_err()).contains("Conflict"));

    zk.disconnect().unwrap();
    server_handle.join().unwrap();
}

#[test]
fn test_get_next_free_uid_mock() {
    let _ = env_logger::builder().is_test(true).try_init();
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind");
    let addr = listener.local_addr().unwrap();
    let port = addr.port();

    let server_handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let session_id = 1111;

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
                    stream
                        .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                        .unwrap();
                }
                CMD_GET_FREE_SIZES => {
                    let mut bytes = vec![0u8; 80];
                    <LittleEndian as ByteOrder>::write_i32(&mut bytes[16..20], 2); // 2 users
                    let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, bytes);
                    stream
                        .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                        .unwrap();
                }
                _CMD_PREPARE_BUFFER => {
                    let mut res_payload = vec![0u8; 5];
                    res_payload[0] = 1;
                    <LittleEndian as ByteOrder>::write_u32(&mut res_payload[1..5], 60); // 4 + 2 * 28
                    let res =
                        ZKPacket::new(CMD_PREPARE_DATA, session_id, packet.reply_id, res_payload);
                    stream
                        .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                        .unwrap();
                }
                _CMD_READ_BUFFER => {
                    let mut data = Vec::new();
                    data.write_i32::<LittleEndian>(56).unwrap();
                    // User 1: UID 1
                    data.write_u16::<LittleEndian>(1).unwrap();
                    data.write_all(&[0u8; 26]).unwrap();
                    // User 2: UID 10
                    data.write_u16::<LittleEndian>(10).unwrap();
                    data.write_all(&[0u8; 26]).unwrap();

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
    zk.connect(ZKProtocol::TCP).unwrap();

    // With UID 1 and 10 used:
    // start_uid 1 -> should return 2
    assert_eq!(zk.get_next_free_uid(1).unwrap(), 2);
    // start_uid 10 -> should return 11
    assert_eq!(zk.get_next_free_uid(10).unwrap(), 11);

    zk.disconnect().unwrap();
    server_handle.join().unwrap();
}

#[test]
fn test_find_user_by_id_mock() {
    let _ = env_logger::builder().is_test(true).try_init();
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind");
    let addr = listener.local_addr().unwrap();
    let port = addr.port();

    let server_handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let session_id = 2222;

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
                    stream
                        .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                        .unwrap();
                }
                CMD_GET_FREE_SIZES => {
                    let mut bytes = vec![0u8; 80];
                    <LittleEndian as ByteOrder>::write_i32(&mut bytes[16..20], 1);
                    let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, bytes);
                    stream
                        .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                        .unwrap();
                }
                _CMD_PREPARE_BUFFER => {
                    let mut res_payload = vec![0u8; 5];
                    res_payload[0] = 1;
                    <LittleEndian as ByteOrder>::write_u32(&mut res_payload[1..5], 32);
                    let res =
                        ZKPacket::new(CMD_PREPARE_DATA, session_id, packet.reply_id, res_payload);
                    stream
                        .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                        .unwrap();
                }
                _CMD_READ_BUFFER => {
                    let mut data = Vec::new();
                    data.write_i32::<LittleEndian>(28).unwrap();
                    data.write_u16::<LittleEndian>(1).unwrap();
                    data.write_all(&[0u8; 22]).unwrap();
                    data.write_i32::<LittleEndian>(12345).unwrap(); // ID "12345"

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
    zk.connect(ZKProtocol::TCP).unwrap();

    let user = zk
        .find_user_by_id("12345")
        .unwrap()
        .expect("User should be found");
    assert_eq!(user.uid, 1);
    assert_eq!(user.user_id, "12345");

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
        let session_id = 3333;

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
                    stream
                        .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                        .unwrap();
                }
                CMD_DELETE_USER => {
                    let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, vec![]);
                    stream
                        .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                        .unwrap();
                }
                CMD_REFRESHDATA => {
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
    zk.connect(ZKProtocol::TCP).unwrap();

    let result = zk.delete_user(1);
    assert!(result.is_ok());

    zk.disconnect().unwrap();
    server_handle.join().unwrap();
}

#[test]
fn test_set_users_bulk_mock() {
    let _ = env_logger::builder().is_test(true).try_init();
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind");
    let addr = listener.local_addr().unwrap();
    let port = addr.port();

    let server_handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let session_id = 4444;

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
                    stream
                        .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                        .unwrap();
                }
                CMD_GET_FREE_SIZES => {
                    let mut bytes = vec![0u8; 80];
                    <LittleEndian as ByteOrder>::write_i32(&mut bytes[16..20], 0);
                    let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, bytes);
                    stream
                        .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                        .unwrap();
                }
                _CMD_PREPARE_BUFFER => {
                    // Empty list for get_users
                    let mut res_payload = vec![0u8; 5];
                    res_payload[0] = 1;
                    <LittleEndian as ByteOrder>::write_u32(&mut res_payload[1..5], 4);
                    let res =
                        ZKPacket::new(CMD_PREPARE_DATA, session_id, packet.reply_id, res_payload);
                    stream
                        .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                        .unwrap();
                }
                _CMD_READ_BUFFER => {
                    let mut data = Vec::new();
                    data.write_i32::<LittleEndian>(0).unwrap();
                    let res = ZKPacket::new(CMD_DATA, session_id, packet.reply_id, data);
                    stream
                        .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                        .unwrap();
                }
                CMD_USER_WRQ => {
                    let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, vec![]);
                    stream
                        .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                        .unwrap();
                }
                CMD_REFRESHDATA => {
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
    zk.connect(ZKProtocol::TCP).unwrap();

    let users = vec![
        User {
            uid: 1,
            user_id: "101".to_string(),
            name: "User 1".to_string(),
            privilege: USER_DEFAULT,
            password: "".to_string(),
            group_id: "1".to_string(),
            card: 0,
        },
        User {
            uid: 2,
            user_id: "102".to_string(),
            name: "User 2".to_string(),
            privilege: USER_DEFAULT,
            password: "".to_string(),
            group_id: "1".to_string(),
            card: 0,
        },
    ];

    let result = zk.set_users_bulk(&users);
    assert!(result.is_ok());

    // Test internal batch conflict
    let conflict_batch = vec![
        User {
            uid: 3,
            user_id: "103".to_string(),
            name: "User 3".to_string(),
            privilege: USER_DEFAULT,
            password: "".to_string(),
            group_id: "1".to_string(),
            card: 0,
        },
        User {
            uid: 4,
            user_id: "103".to_string(), // SAME ID
            name: "User 4".to_string(),
            privilege: USER_DEFAULT,
            password: "".to_string(),
            group_id: "1".to_string(),
            card: 0,
        },
    ];

    let result = zk.set_users_bulk(&conflict_batch);
    assert!(result.is_err());
    assert!(format!("{}", result.unwrap_err()).contains("Conflict in batch"));

    zk.disconnect().unwrap();
    server_handle.join().unwrap();
}
