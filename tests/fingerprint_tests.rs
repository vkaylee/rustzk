mod common;
use byteorder::{ByteOrder, LittleEndian, WriteBytesExt};
use common::{make_prepare_data_payload, with_state, MockZKServer};
use rustzk::constants::*;
use rustzk::models::{Finger, User};
use rustzk::{ZKProtocol, ZK};
use std::io::Write;

// ── test_get_templates_mock ───────────────────────────────────────────────

#[test]
fn test_get_templates_mock() {
    let _ = env_logger::builder().is_test(true).try_init();

    let (server, port) = MockZKServer::new()
        .with_session(1234)
        .on(CMD_GET_FREE_SIZES, |_rid, _| {
            let mut bytes = vec![0u8; 80];
            <LittleEndian as ByteOrder>::write_i32(&mut bytes[24..28], 1); // 1 finger
            Some((CMD_ACK_OK, bytes))
        })
        .on(_CMD_PREPARE_BUFFER, |_rid, _| {
            Some((CMD_PREPARE_DATA, make_prepare_data_payload(16)))
        })
        .on(_CMD_READ_BUFFER, |_rid, _| {
            let mut data = Vec::new();
            data.write_i32::<LittleEndian>(12).unwrap();
            data.write_u16::<LittleEndian>(12).unwrap();
            data.write_u16::<LittleEndian>(1).unwrap();
            data.write_u8(0).unwrap(); // fid
            data.write_u8(1).unwrap(); // valid
            data.write_all(&[0xAA; 6]).unwrap();
            Some((CMD_DATA, data))
        })
        .spawn();

    let mut zk = ZK::new("127.0.0.1", port);
    zk.connect(ZKProtocol::TCP).unwrap();

    let templates = zk.get_templates().unwrap();
    assert_eq!(templates.len(), 1);
    assert_eq!(templates[0].uid(), 1);
    assert_eq!(templates[0].fid(), 0);
    assert_eq!(templates[0].template(), vec![0xAA; 6]);

    zk.disconnect().unwrap();
    server.join();
}

// ── test_delete_user_template_mock ────────────────────────────────────────

#[test]
fn test_delete_user_template_mock() {
    let _ = env_logger::builder().is_test(true).try_init();

    let (server, port) = MockZKServer::new()
        .with_session(5678)
        .on(CMD_DELETE_USERTEMP, |_rid, payload| {
            assert_eq!(payload.len(), 3);
            Some((CMD_ACK_OK, vec![]))
        })
        .on(CMD_REFRESHDATA, |_rid, _| Some((CMD_ACK_OK, vec![])))
        .spawn();

    let mut zk = ZK::new("127.0.0.1", port);
    zk.connect(ZKProtocol::TCP).unwrap();

    let result = zk.delete_user_template(1, 0);
    assert!(result.is_ok());

    zk.disconnect().unwrap();
    server.join();
}

// ── test_save_user_template_mock ──────────────────────────────────────────

#[test]
fn test_save_user_template_mock() {
    let _ = env_logger::builder().is_test(true).try_init();

    // Captures the staged upload buffer reassembled from CMD_DATA chunks.
    let uploaded = with_state(Vec::<u8>::new());
    let cap = uploaded.clone();

    let saw_commit = with_state(false);
    let commit_seen = saw_commit.clone();

    let (server, port) = MockZKServer::new()
        .with_session(4321)
        .on(CMD_FREE_DATA, |_rid, _| Some((CMD_ACK_OK, vec![])))
        .on(CMD_PREPARE_DATA, |_rid, payload| {
            // Host announces total buffer size as a u32.
            assert_eq!(payload.len(), 4);
            Some((CMD_ACK_OK, vec![]))
        })
        .on(CMD_DATA, move |_rid, payload| {
            cap.lock().unwrap().extend_from_slice(payload);
            Some((CMD_ACK_OK, vec![]))
        })
        .on(_CMD_SAVE_USERTEMPS, move |_rid, payload| {
            // Commit payload: pack('<IHH', 12, 0, 8).
            assert_eq!(payload.len(), 8);
            assert_eq!(LittleEndian::read_u32(&payload[0..4]), 12);
            assert_eq!(LittleEndian::read_u16(&payload[4..6]), 0);
            assert_eq!(LittleEndian::read_u16(&payload[6..8]), 8);
            *commit_seen.lock().unwrap() = true;
            Some((CMD_ACK_OK, vec![]))
        })
        .on(CMD_REFRESHDATA, |_rid, _| Some((CMD_ACK_OK, vec![])))
        .spawn();

    let mut zk = ZK::new("127.0.0.1", port);
    zk.connect(ZKProtocol::TCP).unwrap();

    let user = User::new(
        7,
        "Fp User".to_string(),
        USER_DEFAULT,
        String::new(),
        "1".to_string(),
        "700".to_string(),
        0,
    );
    let template = vec![0xABu8; 10];
    let finger = Finger::new(7, 3, 1, template.clone());

    zk.save_user_template(&user, std::slice::from_ref(&finger))
        .unwrap();

    zk.disconnect().unwrap();
    server.join();

    assert!(*saw_commit.lock().unwrap(), "commit command not received");

    let buf = uploaded.lock().unwrap();
    // Default packet size is 28 (small) → upack is 28 bytes.
    // table = 8 bytes (one finger); fpack = 2-byte prefix + 10 template = 12.
    let upack_len = LittleEndian::read_u32(&buf[0..4]) as usize;
    let table_len = LittleEndian::read_u32(&buf[4..8]) as usize;
    let fpack_len = LittleEndian::read_u32(&buf[8..12]) as usize;
    assert_eq!(upack_len, USER_PACKET_SIZE_SMALL);
    assert_eq!(table_len, 8);
    assert_eq!(fpack_len, 12);
    assert_eq!(buf.len(), 12 + upack_len + table_len + fpack_len);

    // Table entry: [flag=2][uid u16][0x10 | fid][offset u32].
    let table = &buf[12 + upack_len..12 + upack_len + table_len];
    assert_eq!(table[0], 2);
    assert_eq!(LittleEndian::read_u16(&table[1..3]), 7);
    assert_eq!(table[3], 0x10 | 3);
    assert_eq!(LittleEndian::read_u32(&table[4..8]), 0);

    // fpack: [u16 size = template_len + 6][template bytes].
    let fpack = &buf[12 + upack_len + table_len..];
    assert_eq!(LittleEndian::read_u16(&fpack[0..2]), (10 + 6) as u16);
    assert_eq!(&fpack[2..], &template[..]);
}

// ── test_save_user_template_multiple_fingers_mock ─────────────────────────

#[test]
fn test_save_user_template_multiple_fingers_mock() {
    let _ = env_logger::builder().is_test(true).try_init();

    let uploaded = with_state(Vec::<u8>::new());
    let cap = uploaded.clone();

    let (server, port) = MockZKServer::new()
        .with_session(4322)
        .on(CMD_FREE_DATA, |_rid, _| Some((CMD_ACK_OK, vec![])))
        .on(CMD_PREPARE_DATA, |_rid, _| Some((CMD_ACK_OK, vec![])))
        .on(CMD_DATA, move |_rid, payload| {
            cap.lock().unwrap().extend_from_slice(payload);
            Some((CMD_ACK_OK, vec![]))
        })
        .on(_CMD_SAVE_USERTEMPS, |_rid, _| Some((CMD_ACK_OK, vec![])))
        .on(CMD_REFRESHDATA, |_rid, _| Some((CMD_ACK_OK, vec![])))
        .spawn();

    let mut zk = ZK::new("127.0.0.1", port);
    zk.connect(ZKProtocol::TCP).unwrap();

    let user = User::new(
        9,
        "Multi".to_string(),
        USER_DEFAULT,
        String::new(),
        "1".to_string(),
        "900".to_string(),
        0,
    );
    // Two fingers with different template lengths to exercise offset accumulation.
    let t0 = vec![0x11u8; 4];
    let t1 = vec![0x22u8; 7];
    let fingers = vec![
        Finger::new(9, 0, 1, t0.clone()),
        Finger::new(9, 1, 1, t1.clone()),
    ];

    zk.save_user_template(&user, &fingers).unwrap();

    zk.disconnect().unwrap();
    server.join();

    let buf = uploaded.lock().unwrap();
    let upack_len = LittleEndian::read_u32(&buf[0..4]) as usize;
    let table_len = LittleEndian::read_u32(&buf[4..8]) as usize;
    let fpack_len = LittleEndian::read_u32(&buf[8..12]) as usize;

    // Two table entries (8 bytes each).
    assert_eq!(table_len, 16);
    // fpack = (2 + 4) + (2 + 7) = 15.
    assert_eq!(fpack_len, 15);

    let table = &buf[12 + upack_len..12 + upack_len + table_len];
    // Entry 0: fid 0, offset 0.
    assert_eq!(table[0], 2);
    assert_eq!(LittleEndian::read_u16(&table[1..3]), 9);
    assert_eq!(table[3], 0x10 | 0);
    assert_eq!(LittleEndian::read_u32(&table[4..8]), 0);
    // Entry 1: fid 1, offset = len of first repack_only block = 2 + 4 = 6.
    assert_eq!(table[8], 2);
    assert_eq!(LittleEndian::read_u16(&table[9..11]), 9);
    assert_eq!(table[11], 0x10 | 1);
    assert_eq!(LittleEndian::read_u32(&table[12..16]), 6);

    // fpack: first block then second block, each [u16 size][template].
    let fpack = &buf[12 + upack_len + table_len..];
    assert_eq!(LittleEndian::read_u16(&fpack[0..2]), (4 + 6) as u16);
    assert_eq!(&fpack[2..6], &t0[..]);
    assert_eq!(LittleEndian::read_u16(&fpack[6..8]), (7 + 6) as u16);
    assert_eq!(&fpack[8..15], &t1[..]);
}

// ── test_save_user_template_empty_fingers ─────────────────────────────────

#[test]
fn test_save_user_template_empty_fingers() {
    let _ = env_logger::builder().is_test(true).try_init();

    // no_default: any command beyond CONNECT/EXIT panics. Since an empty
    // fingers slice must short-circuit, the device should receive nothing.
    let (server, port) = MockZKServer::new()
        .with_session(4323)
        .no_default()
        .on(CMD_CONNECT, |_rid, _| Some((CMD_ACK_OK, vec![])))
        .spawn();

    let mut zk = ZK::new("127.0.0.1", port);
    zk.connect(ZKProtocol::TCP).unwrap();

    let user = User::new(
        1,
        "Empty".to_string(),
        USER_DEFAULT,
        String::new(),
        "1".to_string(),
        "1".to_string(),
        0,
    );

    // Should return Ok without sending any upload/commit command.
    assert!(zk.save_user_template(&user, &[]).is_ok());

    zk.disconnect().unwrap();
    server.join();
}
