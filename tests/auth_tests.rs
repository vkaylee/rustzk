mod common;
use common::MockZKServer;
use rustzk::constants::*;
use rustzk::{ZKProtocol, ZK};

// ── test_auth_handshake_mock ──────────────────────────────────────────────

#[test]
fn test_auth_handshake_mock() {
    let (server, port) = MockZKServer::new()
        .with_session(9876)
        .on(CMD_CONNECT, |_rid, _| Some((CMD_ACK_UNAUTH, vec![])))
        .on(CMD_AUTH, |_rid, payload| {
            if payload.len() == 4 {
                Some((CMD_ACK_OK, vec![]))
            } else {
                Some((CMD_ACK_ERROR, vec![]))
            }
        })
        .spawn();

    let mut zk = ZK::new("127.0.0.1", port);
    zk.set_password(0);

    let res = zk.connect(ZKProtocol::TCP);
    assert!(res.is_ok(), "Connection with auth should succeed");
    assert!(zk.is_connected(), "ZK should be marked as connected");

    zk.disconnect().unwrap();
    server.join();
}

// ── test_connect_with_password_mock ──────────────────────────────────────

#[test]
fn test_connect_with_password_mock() {
    let _ = env_logger::builder().is_test(true).try_init();

    let (server, port) = MockZKServer::new()
        .with_session(5555)
        .on(CMD_CONNECT, |_rid, _| Some((CMD_ACK_UNAUTH, vec![])))
        .on(CMD_AUTH, |_rid, payload| {
            assert_eq!(payload.len(), 4);
            Some((CMD_ACK_OK, vec![]))
        })
        .spawn();

    let mut zk = ZK::new("127.0.0.1", port);
    zk.set_password(123456);

    let result = zk.connect(ZKProtocol::TCP);
    assert!(result.is_ok());
    assert!(zk.is_connected());

    zk.disconnect().unwrap();
    server.join();
}

// ── test_change_password_mock ────────────────────────────────────────────

#[test]
fn test_change_password_mock() {
    let _ = env_logger::builder().is_test(true).try_init();

    let (server, port) = MockZKServer::new()
        .with_session(1234)
        .on(CMD_OPTIONS_WRQ, |_rid, payload| {
            let p = String::from_utf8_lossy(payload);
            assert!(p.contains("ComKey=654321"));
            Some((CMD_ACK_OK, vec![]))
        })
        .spawn();

    let mut zk = ZK::new("127.0.0.1", port);
    zk.connect(ZKProtocol::TCP).unwrap();

    let result = zk.change_password(654321);
    assert!(result.is_ok());

    zk.disconnect().unwrap();
    server.join();
}
