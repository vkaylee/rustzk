mod common;
use common::{make_prepare_data_payload, MockZKServer};
use byteorder::{ByteOrder, LittleEndian, WriteBytesExt};
use rustzk::constants::*;
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
            data.write_u8(0).unwrap();  // fid
            data.write_u8(1).unwrap();  // valid
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
