use rustzk::constants::*;
use rustzk::protocol::{TCPWrapper, ZKPacket};
use rustzk::{ZKError, ZKProtocol, ZK};
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
        assert_eq!(packet.command(), CMD_CONNECT);

        // Send invalid response (not CMD_ACK_OK or CMD_ACK_UNAUTH)
        let res = ZKPacket::new(CMD_ACK_ERROR, 0, packet.reply_id(), vec![]);
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

            match packet.command() {
                CMD_CONNECT => {
                    let res = ZKPacket::new(CMD_ACK_OK, 5555, packet.reply_id(), vec![]);
                    stream
                        .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                        .unwrap();
                }
                CMD_EXIT => {
                    let res = ZKPacket::new(CMD_ACK_OK, 5555, packet.reply_id(), vec![]);
                    let _ = stream.write_all(&TCPWrapper::wrap(&res.to_bytes()));
                    break;
                }
                _ => {}
            }
        }
    });

    // Reconnect to new server — should succeed since transport was cleaned
    zk.set_addr(format!("127.0.0.1:{}", port2));
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

            if packet.command() == CMD_CONNECT {
                let res = ZKPacket::new(CMD_ACK_OK, 1234, packet.reply_id(), vec![]);
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
    zk.set_timeout(Duration::from_secs(2)); // Short timeout for fast test
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

            if packet.command() == CMD_CONNECT {
                let res = ZKPacket::new(CMD_ACK_OK, 1234, packet.reply_id(), vec![]);
                stream
                    .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                    .unwrap();
                tx.send(()).unwrap();
                break;
            }
        }
    });

    let mut zk = ZK::new("127.0.0.1", port);
    zk.set_timeout(Duration::from_secs(2)); // Short timeout for fast test
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

/// Test: reply_id is NOT mutated when calling a command without a transport.
/// Uses restart() which internally calls send_command().
#[test]
fn test_reply_id_stable_without_transport() {
    let mut zk = ZK::new("127.0.0.1", 4370);
    // Not connected — transport is None
    let reply_id_before = zk.reply_id();

    // restart() calls send_command() internally
    let result = zk.restart();
    assert!(result.is_err(), "restart should fail without transport");

    assert_eq!(
        zk.reply_id(),
        reply_id_before,
        "reply_id should NOT change when transport is None"
    );
}

/// Test: finish_handshake() resets session_id to 0 on authentication failure.
#[test]
fn test_auth_failure_resets_session_id() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind");
    let addr = listener.local_addr().unwrap();
    let port = addr.port();

    thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("Failed to accept");

        loop {
            let mut header = [0u8; 8];
            if stream.read_exact(&mut header).is_err() {
                break;
            }
            let (length, _) = TCPWrapper::decode_header(&header).unwrap();
            let mut body = vec![0u8; length];
            stream.read_exact(&mut body).unwrap();
            let packet = ZKPacket::from_bytes(&body).unwrap();

            match packet.command() {
                CMD_CONNECT => {
                    // Respond with CMD_ACK_UNAUTH (password required)
                    let res = ZKPacket::new(CMD_ACK_UNAUTH, 9999, packet.reply_id(), vec![]);
                    stream
                        .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                        .unwrap();
                }
                CMD_AUTH => {
                    // Respond with CMD_ACK_UNAUTH (wrong password)
                    let res = ZKPacket::new(CMD_ACK_UNAUTH, 9999, packet.reply_id(), vec![]);
                    stream
                        .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                        .unwrap();
                }
                _ => break,
            }
        }
    });

    let mut zk = ZK::new("127.0.0.1", port);
    zk.set_password(12345); // Wrong password

    let result = zk.connect(ZKProtocol::TCP);
    assert!(result.is_err(), "Connect should fail with wrong password");
    assert!(!zk.is_connected());
    assert_eq!(
        zk.session_id(),
        0,
        "session_id should be reset to 0 after auth failure"
    );
}

/// Test: Auto fallback preserves use_legacy_checksum when TCP fails.
#[test]
fn test_auto_fallback_preserves_checksum_flag() {
    // Bind a TCP port that will refuse connections by immediately dropping
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind");
    let addr = listener.local_addr().unwrap();
    let port = addr.port();

    // Accept and immediately close — simulates a TCP connect that fails handshake
    thread::spawn(move || {
        // Accept connection but send invalid response to make handshake fail
        if let Ok((mut stream, _)) = listener.accept() {
            let mut header = [0u8; 8];
            if stream.read_exact(&mut header).is_ok() {
                let (length, _) = TCPWrapper::decode_header(&header).unwrap();
                let mut body = vec![0u8; length];
                let _ = stream.read_exact(&mut body);
                if let Ok(packet) = ZKPacket::from_bytes(&body) {
                    // Send an error response
                    let res = ZKPacket::new(CMD_ACK_ERROR, 0, packet.reply_id(), vec![]);
                    let _ = stream.write_all(&TCPWrapper::wrap(&res.to_bytes()));
                }
            }
        }
    });

    let mut zk = ZK::new("127.0.0.1", port);
    zk.set_legacy_checksum(true); // Explicitly set legacy

    // Auto will try TCP first (which fails), then fall back to UDP (which also fails,
    // but the important thing is the checksum flag should be preserved)
    let _result = zk.connect(ZKProtocol::Auto);

    assert!(
        zk.use_legacy_checksum(),
        "use_legacy_checksum should be preserved after Auto fallback, not flipped by failed TCP handshake"
    );
}

/// Test: ZKError helper methods classify errors correctly.
#[test]
fn test_zkerror_helper_methods() {
    use std::io;

    let timeout_io = ZKError::Network(io::Error::new(io::ErrorKind::TimedOut, "timeout"));
    let would_block_io = ZKError::Network(io::Error::new(io::ErrorKind::WouldBlock, "would block"));
    let connection_timeout = ZKError::Connection(
        rustzk::ZKErrorCode::Timeout,
        "Connection timeout occurred".into(),
    );
    let response_timeout = ZKError::Response(
        rustzk::ZKErrorCode::Timeout,
        "TimedOut waiting for packet".into(),
    );
    let _unrelated_io = ZKError::Network(io::Error::new(io::ErrorKind::PermissionDenied, "denied"));

    assert!(timeout_io.is_timeout());
    assert!(would_block_io.is_timeout());
    assert!(connection_timeout.is_timeout());
    assert!(response_timeout.is_timeout());

    let unauthorized_conn = ZKError::Connection(
        rustzk::ZKErrorCode::Unauthorized,
        "Unauthorized: Password required or incorrect".into(),
    );
    let unrelated_conn = ZKError::Connection(
        rustzk::ZKErrorCode::ConnectionFailed,
        "Network unreachable".into(),
    );

    assert!(unauthorized_conn.is_unauthorized());
    assert!(!unrelated_conn.is_unauthorized());
}

/// Test: is_alive() doesn't hang when connection is dead.
#[test]
fn test_zk_is_alive_timeout_handling() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind");
    let addr = listener.local_addr().unwrap();
    let port = addr.port();

    thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("Failed to accept");

        // Handle connect
        let mut header = [0u8; 8];
        if stream.read_exact(&mut header).is_ok() {
            let (length, _) = TCPWrapper::decode_header(&header).unwrap();
            let mut body = vec![0u8; length];
            let _ = stream.read_exact(&mut body);
            let packet = ZKPacket::from_bytes(&body).unwrap();
            let res = ZKPacket::new(CMD_ACK_OK, 1234, packet.reply_id(), vec![]);
            let _ = stream.write_all(&TCPWrapper::wrap(&res.to_bytes()));
        }

        // For subsequent requests (like CMD_GET_TIME in is_alive), we just do nothing
        // to simulate a silent/hung connection
        thread::sleep(Duration::from_secs(5));
    });

    let mut zk = ZK::new("127.0.0.1", port);
    zk.connect(ZKProtocol::TCP).unwrap();
    assert!(zk.is_connected());

    // is_alive should fail quickly (timeout set to 2s in implementation),
    // and reset the connection state
    let start = std::time::Instant::now();
    let alive = zk.is_alive();
    let elapsed = start.elapsed();

    assert!(!alive);
    assert!(!zk.is_connected());
    assert!(
        elapsed < Duration::from_secs(4),
        "is_alive should time out in ~2s, took {:?}",
        elapsed
    );
}

/// Test: TCP stream recovery immediately closes transport on framing error.
#[test]
fn test_tcp_framing_deserialization_recovery() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind");
    let addr = listener.local_addr().unwrap();
    let port = addr.port();

    thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("Failed to accept");

        // Handle connect
        let mut header = [0u8; 8];
        if stream.read_exact(&mut header).is_ok() {
            let (length, _) = TCPWrapper::decode_header(&header).unwrap();
            let mut body = vec![0u8; length];
            let _ = stream.read_exact(&mut body);
            let packet = ZKPacket::from_bytes(&body).unwrap();
            let res = ZKPacket::new(CMD_ACK_OK, 1234, packet.reply_id(), vec![]);
            let _ = stream.write_all(&TCPWrapper::wrap(&res.to_bytes()));
        }

        // Send corrupt header magic bytes back for the next command
        // Wrong magic: [0x00, 0x00, 0x00, 0x00]
        let corrupt_header = [0u8; 8];
        let _ = stream.write_all(&corrupt_header);
    });

    let mut zk = ZK::new("127.0.0.1", port);
    zk.connect(ZKProtocol::TCP).unwrap();
    assert!(zk.is_connected());

    // Send option query, which will read the corrupt header
    let res = zk.get_option_value("TZAdj");
    assert!(res.is_err());
    assert!(!zk.is_connected(), "Connection should be marked offline");
}

/// Test: Drop is completely non-blocking.
#[test]
fn test_zk_drop_non_blocking() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind");
    let addr = listener.local_addr().unwrap();
    let port = addr.port();

    thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("Failed to accept");
        // Handle connect
        let mut header = [0u8; 8];
        if stream.read_exact(&mut header).is_ok() {
            let (length, _) = TCPWrapper::decode_header(&header).unwrap();
            let mut body = vec![0u8; length];
            let _ = stream.read_exact(&mut body);
            let packet = ZKPacket::from_bytes(&body).unwrap();
            let res = ZKPacket::new(CMD_ACK_OK, 1234, packet.reply_id(), vec![]);
            let _ = stream.write_all(&TCPWrapper::wrap(&res.to_bytes()));
        }
        // Sleep to simulate unreachable connection during drop
        thread::sleep(Duration::from_secs(5));
    });

    let mut zk = ZK::new("127.0.0.1", port);
    zk.connect(ZKProtocol::TCP).unwrap();

    let start = std::time::Instant::now();
    // Drop the ZK instance. It should be non-blocking (microsecond scale),
    // and not block for 3 seconds or 5 seconds.
    std::mem::drop(zk);
    let elapsed = start.elapsed();

    assert!(
        elapsed < Duration::from_millis(50),
        "Drop took {:?}, should be near-instant (non-blocking)",
        elapsed
    );
}
