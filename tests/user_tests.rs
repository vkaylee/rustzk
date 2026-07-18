mod common;
use byteorder::{ByteOrder, LittleEndian, WriteBytesExt};
use common::{make_prepare_data_payload, MockZKServer};
use rustzk::constants::*;
use rustzk::models::User;
use rustzk::{ZKProtocol, ZK};
use std::io::Write;

// ── test_set_user_mock ────────────────────────────────────────────────────

#[test]
fn test_set_user_mock() {
    let _ = env_logger::builder().is_test(true).try_init();

    let (server, port) = MockZKServer::new()
        .with_session(8888)
        .on(CMD_GET_FREE_SIZES, |_rid, _| {
            let mut bytes = Vec::new();
            for _ in 0..20 {
                bytes.write_i32::<LittleEndian>(0).unwrap();
            }
            Some((CMD_ACK_OK, bytes))
        })
        .on(_CMD_PREPARE_BUFFER, |_rid, _| {
            Some((CMD_PREPARE_DATA, vec![1, 4, 0, 0, 0]))
        })
        .on(_CMD_READ_BUFFER, |_rid, _| {
            Some((CMD_DATA, vec![0, 0, 0, 0]))
        })
        .on(CMD_USER_WRQ, |_rid, _| Some((CMD_ACK_OK, vec![])))
        .on(CMD_REFRESHDATA, |_rid, _| Some((CMD_ACK_OK, vec![])))
        .spawn();

    let mut zk = ZK::new("127.0.0.1", port);
    zk.connect(ZKProtocol::TCP).unwrap();

    let user = User::new(
        1,
        "Test User".to_string(),
        USER_ADMIN,
        "123".to_string(),
        "1".to_string(),
        "101".to_string(),
        0,
    );
    let result = zk.set_user(&user);
    assert!(result.is_ok());

    zk.disconnect().unwrap();
    server.join();
}

// ── test_set_user_conflict_mock ───────────────────────────────────────────

#[test]
fn test_set_user_conflict_mock() {
    let _ = env_logger::builder().is_test(true).try_init();

    let (server, port) = MockZKServer::new()
        .with_session(7777)
        .on(CMD_GET_FREE_SIZES, |_rid, _| {
            let mut bytes = vec![0u8; 80];
            <LittleEndian as ByteOrder>::write_i32(&mut bytes[16..20], 1);
            Some((CMD_ACK_OK, bytes))
        })
        .on(_CMD_PREPARE_BUFFER, |_rid, _| {
            Some((CMD_PREPARE_DATA, make_prepare_data_payload(32)))
        })
        .on(_CMD_READ_BUFFER, |_rid, _| {
            let mut data = Vec::new();
            data.write_i32::<LittleEndian>(28).unwrap();
            data.write_u16::<LittleEndian>(10).unwrap();
            data.write_u8(0).unwrap();
            data.write_all(&[0u8; 5]).unwrap();
            data.write_all(&[0u8; 8]).unwrap();
            data.write_u32::<LittleEndian>(0).unwrap();
            data.write_u8(0).unwrap();
            data.write_u8(1).unwrap();
            data.write_u16::<LittleEndian>(0).unwrap();
            data.write_i32::<LittleEndian>(101).unwrap();
            Some((CMD_DATA, data))
        })
        .spawn();

    let mut zk = ZK::new("127.0.0.1", port);
    zk.connect(ZKProtocol::TCP).unwrap();

    let user = User::new(
        11,
        "Cloned User".to_string(),
        USER_DEFAULT,
        "".to_string(),
        "1".to_string(),
        "101".to_string(),
        0,
    );
    let result = zk.set_user(&user);
    assert!(result.is_err());
    assert!(format!("{}", result.unwrap_err()).contains("Conflict"));

    zk.disconnect().unwrap();
    server.join();
}

// ── test_get_next_free_uid_mock ───────────────────────────────────────────

#[test]
fn test_get_next_free_uid_mock() {
    let _ = env_logger::builder().is_test(true).try_init();

    let (server, port) = MockZKServer::new()
        .with_session(1111)
        .on(CMD_GET_FREE_SIZES, |_rid, _| {
            let mut bytes = vec![0u8; 80];
            <LittleEndian as ByteOrder>::write_i32(&mut bytes[16..20], 2);
            Some((CMD_ACK_OK, bytes))
        })
        .on(_CMD_PREPARE_BUFFER, |_rid, _| {
            Some((CMD_PREPARE_DATA, make_prepare_data_payload(60)))
        })
        .on(_CMD_READ_BUFFER, |_rid, _| {
            let mut data = Vec::new();
            data.write_i32::<LittleEndian>(56).unwrap();
            data.write_u16::<LittleEndian>(1).unwrap();
            data.write_all(&[0u8; 26]).unwrap();
            data.write_u16::<LittleEndian>(10).unwrap();
            data.write_all(&[0u8; 26]).unwrap();
            Some((CMD_DATA, data))
        })
        .spawn();

    let mut zk = ZK::new("127.0.0.1", port);
    zk.connect(ZKProtocol::TCP).unwrap();

    assert_eq!(zk.get_next_free_uid(1).unwrap(), 2);
    assert_eq!(zk.get_next_free_uid(10).unwrap(), 11);

    zk.disconnect().unwrap();
    server.join();
}

// ── test_find_user_by_id_mock ─────────────────────────────────────────────

#[test]
fn test_find_user_by_id_mock() {
    let _ = env_logger::builder().is_test(true).try_init();

    let (server, port) = MockZKServer::new()
        .with_session(2222)
        .on(CMD_GET_FREE_SIZES, |_rid, _| {
            let mut bytes = vec![0u8; 80];
            <LittleEndian as ByteOrder>::write_i32(&mut bytes[16..20], 1);
            Some((CMD_ACK_OK, bytes))
        })
        .on(_CMD_PREPARE_BUFFER, |_rid, _| {
            Some((CMD_PREPARE_DATA, make_prepare_data_payload(32)))
        })
        .on(_CMD_READ_BUFFER, |_rid, _| {
            let mut data = Vec::new();
            data.write_i32::<LittleEndian>(28).unwrap();
            data.write_u16::<LittleEndian>(1).unwrap();
            data.write_all(&[0u8; 22]).unwrap();
            data.write_i32::<LittleEndian>(12345).unwrap();
            Some((CMD_DATA, data))
        })
        .spawn();

    let mut zk = ZK::new("127.0.0.1", port);
    zk.connect(ZKProtocol::TCP).unwrap();

    let user = zk
        .find_user_by_id("12345")
        .unwrap()
        .expect("User should be found");
    assert_eq!(user.uid(), 1);
    assert_eq!(user.user_id(), "12345");

    zk.disconnect().unwrap();
    server.join();
}

// ── test_delete_user_mock ─────────────────────────────────────────────────

#[test]
fn test_delete_user_mock() {
    let _ = env_logger::builder().is_test(true).try_init();

    let (server, port) = MockZKServer::new()
        .with_session(3333)
        .on(CMD_DELETE_USER, |_rid, _| Some((CMD_ACK_OK, vec![])))
        .on(CMD_REFRESHDATA, |_rid, _| Some((CMD_ACK_OK, vec![])))
        .spawn();

    let mut zk = ZK::new("127.0.0.1", port);
    zk.connect(ZKProtocol::TCP).unwrap();

    let result = zk.delete_user(1);
    assert!(result.is_ok());

    zk.disconnect().unwrap();
    server.join();
}

// ── test_set_users_bulk_mock ──────────────────────────────────────────────

#[test]
fn test_set_users_bulk_mock() {
    let _ = env_logger::builder().is_test(true).try_init();

    let (server, port) = MockZKServer::new()
        .with_session(4444)
        .on(CMD_GET_FREE_SIZES, |_rid, _| {
            let mut bytes = vec![0u8; 80];
            <LittleEndian as ByteOrder>::write_i32(&mut bytes[16..20], 0);
            Some((CMD_ACK_OK, bytes))
        })
        .on(_CMD_PREPARE_BUFFER, |_rid, _| {
            Some((CMD_PREPARE_DATA, make_prepare_data_payload(4)))
        })
        .on(_CMD_READ_BUFFER, |_rid, _| {
            let mut data = Vec::new();
            data.write_i32::<LittleEndian>(0).unwrap();
            Some((CMD_DATA, data))
        })
        .on(CMD_USER_WRQ, |_rid, _| Some((CMD_ACK_OK, vec![])))
        .on(CMD_REFRESHDATA, |_rid, _| Some((CMD_ACK_OK, vec![])))
        .spawn();

    let mut zk = ZK::new("127.0.0.1", port);
    zk.connect(ZKProtocol::TCP).unwrap();

    let users = vec![
        User::new(
            1,
            "User 1".into(),
            USER_DEFAULT,
            "".into(),
            "1".into(),
            "101".into(),
            0,
        ),
        User::new(
            2,
            "User 2".into(),
            USER_DEFAULT,
            "".into(),
            "1".into(),
            "102".into(),
            0,
        ),
    ];
    let result = zk.set_users_bulk(&users);
    assert!(result.is_ok());

    let conflict_batch = vec![
        User::new(
            3,
            "User 3".into(),
            USER_DEFAULT,
            "".into(),
            "1".into(),
            "103".into(),
            0,
        ),
        User::new(
            4,
            "User 4".into(),
            USER_DEFAULT,
            "".into(),
            "1".into(),
            "103".into(),
            0,
        ),
    ];
    let result = zk.set_users_bulk(&conflict_batch);
    assert!(result.is_err());
    assert!(format!("{}", result.unwrap_err()).contains("Conflict in batch"));

    zk.disconnect().unwrap();
    server.join();
}

// ── test_user_id_cache_invalidation_and_timezone_redundancy ──────────────

#[test]
fn test_user_id_cache_invalidation_and_timezone_redundancy() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    let _ = env_logger::builder().is_test(true).try_init();

    let get_option_count = Arc::new(AtomicUsize::new(0));
    let get_option_count_clone = Arc::clone(&get_option_count);

    let (server, port) = MockZKServer::new()
        .with_session(5555)
        .on(CMD_GET_FREE_SIZES, |_rid, _| {
            Some((CMD_ACK_OK, vec![0u8; 80]))
        })
        .on(_CMD_PREPARE_BUFFER, |_rid, _| {
            Some((CMD_PREPARE_DATA, make_prepare_data_payload(4)))
        })
        .on(_CMD_READ_BUFFER, |_rid, _| {
            let mut data = Vec::new();
            data.write_i32::<LittleEndian>(0).unwrap();
            Some((CMD_DATA, data))
        })
        .on(CMD_USER_WRQ, |_rid, _| Some((CMD_ACK_OK, vec![])))
        .on(CMD_DELETE_USER, |_rid, _| Some((CMD_ACK_OK, vec![])))
        .on(CMD_REFRESHDATA, |_rid, _| Some((CMD_ACK_OK, vec![])))
        .on(CMD_OPTIONS_RRQ, move |_rid, _| {
            get_option_count_clone.fetch_add(1, Ordering::SeqCst);
            Some((CMD_ACK_UNAUTH, vec![]))
        })
        .on(CMD_GET_TIME, |_rid, _| Some((CMD_ACK_OK, vec![0, 0, 0, 0])))
        .spawn();

    let mut zk = ZK::new("127.0.0.1", port);
    zk.connect(ZKProtocol::TCP).unwrap();

    let user = User::new(
        1,
        "User 1".into(),
        USER_DEFAULT,
        "".into(),
        "1".into(),
        "101".into(),
        0,
    );
    zk.set_user(&user).unwrap();
    assert!(zk.is_connected());

    let _ = zk.get_time();
    assert!(zk.timezone_synced());

    let _ = zk.read_sizes();
    assert_eq!(get_option_count.load(Ordering::SeqCst), 1);

    zk.delete_user(1).unwrap();
    zk.disconnect().unwrap();
    server.join();
}
