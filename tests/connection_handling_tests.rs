use rustzk::constants::*;
use rustzk::protocol::{TCPWrapper, ZKPacket};
use rustzk::{ZKProtocol, ZK};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/// Test: Transport is cleaned up when handshake fails.
/// After a failed connect(), calling connect() again should succeed
/// (proving transport was set to None and is_connected remains false).
#[test]
fn test_transport_cleaned_on_handshake_failure() {
    // First server: accepts TCP but sends an invalid response to CMD_CONNECT
    let listener1 = TcpListener::bind("127.0.0.1:0").expect("Failed to bind mock server 1");
    let addr1 = listener1.local_addr().unwrap();
    let port1 = addr1.port();

    let handle1 = thread::spawn(move || {
        let (mut stream, _) = listener1.accept().expect("Failed to accept connection");

        // Read the CMD_CONNECT
        let mut header = [0u8; 8];
        stream.read_exact(&mut header).unwrap();
        let (length, _) = TCPWrapper::decode_header(&header).unwrap();
        let mut body = vec![0u8; length];
        stream.read_exact(&mut body).unwrap();
        let packet = ZKPacket::from_bytes(&body).unwrap();
        assert_eq!(packet.command, CMD_CONNECT);

        // Send invalid response (not CMD_ACK_OK or CMD_ACK_UNAUTH)
        let res = ZKPacket::new(CMD_ACK_ERROR, 0, packet.reply_id, vec![]);
        stream
            .write_all(&TCPWrapper::wrap(&res.to_bytes()))
            .unwrap();
    });

    let mut zk = ZK::new("127.0.0.1", port1);
    let result = zk.connect(ZKProtocol::TCP);
    assert!(
        result.is_err(),
        "Connect with invalid handshake should fail"
    );
    assert!(
        !zk.is_connected(),
        "Should not be connected after failed handshake"
    );

    handle1.join().unwrap();

    // Second server: proper server for retry
    let listener2 = TcpListener::bind("127.0.0.1:0").expect("Failed to bind mock server 2");
    let addr2 = listener2.local_addr().unwrap();
    let port2 = addr2.port();

    let handle2 = thread::spawn(move || {
        let (mut stream, _) = listener2.accept().expect("Failed to accept connection");

        loop {
            let mut header = [0u8; 8];
            if stream.read_exact(&mut header).is_err() {
                break;
            }
            let (length, _) = TCPWrapper::decode_header(&header).unwrap();
            let mut body = vec![0u8; length];
            stream.read_exact(&mut body).unwrap();
            let packet = ZKPacket::from_bytes(&body).unwrap();

            match packet.command {
                CMD_CONNECT => {
                    let res = ZKPacket::new(CMD_ACK_OK, 5555, packet.reply_id, vec![]);
                    stream
                        .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                        .unwrap();
                }
                CMD_EXIT => {
                    let res = ZKPacket::new(CMD_ACK_OK, 5555, packet.reply_id, vec![]);
                    let _ = stream.write_all(&TCPWrapper::wrap(&res.to_bytes()));
                    break;
                }
                _ => {}
            }
        }
    });

    // Reconnect to new server — should succeed since transport was cleaned
    zk.addr = format!("127.0.0.1:{}", port2);
    let result = zk.connect(ZKProtocol::TCP);
    assert!(
        result.is_ok(),
        "Reconnect should succeed after transport cleanup: {:?}",
        result.err()
    );
    assert!(
        zk.is_connected(),
        "Should be connected after successful retry"
    );

    zk.disconnect().unwrap();
    handle2.join().unwrap();
}

/// Test: restart() resets state even when send_command fails (device unreachable).
#[test]
fn test_restart_resets_state_on_error() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind mock server");
    let addr = listener.local_addr().unwrap();
    let port = addr.port();

    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("Failed to accept connection");

        loop {
            let mut header = [0u8; 8];
            if stream.read_exact(&mut header).is_err() {
                break;
            }
            let (length, _) = TCPWrapper::decode_header(&header).unwrap();
            let mut body = vec![0u8; length];
            stream.read_exact(&mut body).unwrap();
            let packet = ZKPacket::from_bytes(&body).unwrap();

            if packet.command == CMD_CONNECT {
                let res = ZKPacket::new(CMD_ACK_OK, 1234, packet.reply_id, vec![]);
                stream
                    .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                    .unwrap();
                // Signal that connect is done, then close the stream
                // to simulate device becoming unreachable
                tx.send(()).unwrap();
                break;
            }
        }
        // stream drops here — device is now "unreachable"
    });

    let mut zk = ZK::new("127.0.0.1", port);
    zk.timeout = Duration::from_secs(2); // Short timeout for fast test
    zk.connect(ZKProtocol::TCP).expect("Connect should succeed");
    assert!(zk.is_connected());

    rx.recv().unwrap(); // Wait for server to close stream
    thread::sleep(Duration::from_millis(100)); // Give server time to fully close

    // restart() should fail (device unreachable) but STILL reset state
    let result = zk.restart();
    assert!(
        result.is_err(),
        "restart() should fail on unreachable device"
    );
    assert!(
        !zk.is_connected(),
        "is_connected should be false after restart() even on error"
    );
}

/// Test: poweroff() resets state even when send_command fails (device unreachable).
#[test]
fn test_poweroff_resets_state_on_error() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind mock server");
    let addr = listener.local_addr().unwrap();
    let port = addr.port();

    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("Failed to accept connection");

        loop {
            let mut header = [0u8; 8];
            if stream.read_exact(&mut header).is_err() {
                break;
            }
            let (length, _) = TCPWrapper::decode_header(&header).unwrap();
            let mut body = vec![0u8; length];
            stream.read_exact(&mut body).unwrap();
            let packet = ZKPacket::from_bytes(&body).unwrap();

            if packet.command == CMD_CONNECT {
                let res = ZKPacket::new(CMD_ACK_OK, 1234, packet.reply_id, vec![]);
                stream
                    .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                    .unwrap();
                tx.send(()).unwrap();
                break;
            }
        }
    });

    let mut zk = ZK::new("127.0.0.1", port);
    zk.timeout = Duration::from_secs(2); // Short timeout for fast test
    zk.connect(ZKProtocol::TCP).expect("Connect should succeed");
    assert!(zk.is_connected());

    rx.recv().unwrap();
    thread::sleep(Duration::from_millis(100));

    // poweroff() should fail (device unreachable) but STILL reset state
    let result = zk.poweroff();
    assert!(
        result.is_err(),
        "poweroff() should fail on unreachable device"
    );
    assert!(
        !zk.is_connected(),
        "is_connected should be false after poweroff() even on error"
    );
}
