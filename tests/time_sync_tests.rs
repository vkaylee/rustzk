use byteorder::{LittleEndian, WriteBytesExt};
use rustzk::constants::*;
use rustzk::{ZKProtocol, ZK};

mod common;
use common::MockZKServer;

#[test]
fn test_lazy_timezone_sync_on_get_time() {
    let _ = env_logger::builder().is_test(true).try_init();

    let (server, port) = MockZKServer::new()
        .with_session(1234)
        .on(CMD_OPTIONS_RRQ, |_rid, _| {
            Some((CMD_ACK_OK, b"TZAdj=7\0".to_vec()))
        })
        .on(CMD_GET_TIME, |_rid, _| {
            let t: u32 = 839845230;
            let mut payload = Vec::new();
            payload.write_u32::<LittleEndian>(t).unwrap();
            Some((CMD_ACK_OK, payload))
        })
        .spawn();

    let mut zk = ZK::new("127.0.0.1", port);

    // 1. Connect (should NOT sync timezone yet)
    zk.connect(ZKProtocol::TCP).unwrap();
    assert!(!zk.timezone_synced());
    assert_eq!(zk.timezone_offset(), 0);

    // 2. Call get_time (should trigger lazy sync)
    let time = zk.get_time().unwrap();

    // 3. Verify sync happened
    assert!(zk.timezone_synced());
    assert_eq!(zk.timezone_offset(), 7 * 60); // 420 minutes
    assert_eq!(time.timezone().local_minus_utc(), 7 * 3600);

    server.join();
}

#[test]
fn test_lazy_sync_happens_only_once() {
    let _ = env_logger::builder().is_test(true).try_init();

    let (server, port) = MockZKServer::new()
        .with_session(5678)
        .no_default() // Panic on any unexpected command
        .on(CMD_CONNECT, |_rid, _| Some((CMD_ACK_OK, vec![])))
        .on(CMD_OPTIONS_RRQ, |_rid, _| {
            Some((CMD_ACK_OK, b"TZAdj=8\0".to_vec()))
        })
        .on(CMD_GET_TIME, |_rid, _| {
            Some((CMD_ACK_OK, 839845230u32.to_le_bytes().to_vec()))
        })
        .spawn();

    let mut zk = ZK::new("127.0.0.1", port);
    zk.connect(ZKProtocol::TCP).unwrap();

    // First call: Triggers sync
    zk.get_time().unwrap();
    assert!(zk.timezone_synced());
    assert_eq!(zk.timezone_offset(), 8 * 60);

    // Second call: Should reuse cached offset (no second CMD_OPTIONS_RRQ)
    // no_default() ensures any unexpected command will panic
    zk.get_time().unwrap();
    assert!(zk.timezone_synced());

    server.join();
}
