use rustzk::constants::*;
use rustzk::protocol::{TCPWrapper, ZKPacket};
use rustzk::{ZKProtocol, ZK};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

#[test]
fn test_zk_drop_auto_disconnects() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind mock server");
    let addr = listener.local_addr().unwrap();
    let port = addr.port();

    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("Failed to accept connection");
        let mut session_id = 0;

        loop {
            let mut header = [0u8; 8];
            if stream.read_exact(&mut header).is_err() {
                break;
            }

            let (length, _) = TCPWrapper::decode_header(&header).expect("Failed to decode header");
            let mut body = vec![0u8; length];
            stream.read_exact(&mut body).expect("Failed to read body");
            let packet = ZKPacket::from_bytes(&body).expect("Failed to parse packet");

            match packet.command {
                CMD_CONNECT => {
                    session_id = 1234;
                    let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, vec![]);
                    stream
                        .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                        .unwrap();
                }
                CMD_EXIT => {
                    // Signal that we received CMD_EXIT
                    tx.send(true).unwrap();
                    let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, vec![]);
                    let _ = stream.write_all(&TCPWrapper::wrap(&res.to_bytes()));
                    break;
                }
                _ => {}
            }
        }
    });

    {
        let mut zk = ZK::new("127.0.0.1", port);
        zk.connect(ZKProtocol::TCP).expect("Failed to connect");
        assert!(zk.is_connected());
        // zk will be dropped at the end of this block.
        // Drop should auto-disconnect by sending CMD_EXIT.
    }

    // Verify that CMD_EXIT WAS sent by Drop (auto-disconnect)
    let result = rx.recv_timeout(Duration::from_secs(3));
    assert!(
        result.is_ok(),
        "Drop should auto-disconnect by sending CMD_EXIT"
    );
}

#[test]
fn test_zk_manual_disconnect_then_drop() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind mock server");
    let addr = listener.local_addr().unwrap();
    let port = addr.port();

    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("Failed to accept connection");
        let mut session_id = 0;
        let mut exit_count = 0;

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
                    session_id = 1234;
                    let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, vec![]);
                    let _ = stream.write_all(&TCPWrapper::wrap(&res.to_bytes()));
                }
                CMD_EXIT => {
                    exit_count += 1;
                    tx.send(exit_count).unwrap();
                    let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, vec![]);
                    let _ = stream.write_all(&TCPWrapper::wrap(&res.to_bytes()));
                    // Don't break immediately to see if another exit comes
                }
                _ => {}
            }
        }
    });

    {
        let mut zk = ZK::new("127.0.0.1", port);
        zk.connect(ZKProtocol::TCP).unwrap();
        zk.disconnect().unwrap(); // Manual disconnect
        assert!(!zk.is_connected());
        // zk will be dropped here, but it's already disconnected
    }

    // Verify exit was called only once (during manual disconnect)
    let count = rx
        .recv_timeout(Duration::from_secs(1))
        .expect("Did not receive exit");
    assert_eq!(count, 1);

    // Wait a bit more to ensure no second exit arrives from Drop
    assert!(rx.recv_timeout(Duration::from_millis(500)).is_err());
}

#[test]
fn test_zk_double_connect_returns_error() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind mock server");
    let addr = listener.local_addr().unwrap();
    let port = addr.port();

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

            match packet.command {
                CMD_CONNECT => {
                    let res = ZKPacket::new(CMD_ACK_OK, 1234, packet.reply_id, vec![]);
                    let _ = stream.write_all(&TCPWrapper::wrap(&res.to_bytes()));
                }
                CMD_EXIT => {
                    let res = ZKPacket::new(CMD_ACK_OK, 1234, packet.reply_id, vec![]);
                    let _ = stream.write_all(&TCPWrapper::wrap(&res.to_bytes()));
                    break;
                }
                _ => {}
            }
        }
    });

    let mut zk = ZK::new("127.0.0.1", port);
    zk.connect(ZKProtocol::TCP)
        .expect("First connect should succeed");
    assert!(zk.is_connected());

    // Second connect should fail with "Already connected"
    let result = zk.connect(ZKProtocol::TCP);
    assert!(result.is_err(), "Double connect should return an error");
    let err_msg = format!("{}", result.unwrap_err());
    assert!(
        err_msg.contains("Already connected"),
        "Error should mention 'Already connected', got: {}",
        err_msg
    );

    zk.disconnect().unwrap();
}

#[test]
fn test_zk_disconnect_uses_short_timeout() {
    // Connect to a mock server, then close the server to simulate
    // an unreachable device. Disconnect should complete within ~3s,
    // not the full 60s timeout.
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
                let _ = stream.write_all(&TCPWrapper::wrap(&res.to_bytes()));
                // Signal that connect is done, then close the stream
                // to simulate device becoming unreachable
                tx.send(()).unwrap();
                break;
            }
        }
        // stream drops here — device is now "unreachable"
    });

    let mut zk = ZK::new("127.0.0.1", port);
    zk.connect(ZKProtocol::TCP).expect("Connect should succeed");
    rx.recv().unwrap(); // Wait for server to close stream

    // Give the server time to fully close
    thread::sleep(Duration::from_millis(100));

    // Disconnect should complete in ~3s, NOT 60s
    let start = std::time::Instant::now();
    let _ = zk.disconnect();
    let elapsed = start.elapsed();

    assert!(
        elapsed < Duration::from_secs(10),
        "Disconnect took {:?}, should complete within ~3s (short timeout)",
        elapsed
    );
}
