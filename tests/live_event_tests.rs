use byteorder::{LittleEndian, WriteBytesExt};
use rustzk::constants::*;
use rustzk::protocol::{TCPWrapper, ZKPacket};
use rustzk::{ZKProtocol, ZK};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;
use std::time::Duration;

fn start_live_mock_server(listener: TcpListener, session_id: u16, scenario: &'static str) {
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();

        loop {
            let mut header = [0u8; 8];
            if stream.read_exact(&mut header).is_err() {
                break;
            }
            let (length, _) = TCPWrapper::decode_header(&header).unwrap();
            let mut body = vec![0u8; length];
            stream.read_exact(&mut body).unwrap();
            let packet = ZKPacket::from_bytes_owned(body).unwrap();

            match packet.command() {
                CMD_CONNECT => {
                    let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id(), vec![]);
                    stream
                        .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                        .unwrap();
                }
                CMD_REG_EVENT => {
                    let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id(), vec![]);
                    stream
                        .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                        .unwrap();

                    // After REG_EVENT, send the simulated event based on scenario
                    thread::sleep(Duration::from_millis(50));
                    let event_data = match scenario {
                        "10byte" => {
                            let mut d = Vec::new();
                            d.write_u16::<LittleEndian>(101).unwrap();
                            d.push(1);
                            d.push(0);
                            d.extend_from_slice(&[26, 2, 20, 10, 30, 0]);
                            d
                        }
                        "32byte" => {
                            let mut d = vec![0u8; 32];
                            d[..13].copy_from_slice(b"RUST-USER-001");
                            d[24] = 1;
                            d[25] = 0;
                            d[26..32].copy_from_slice(&[26, 2, 20, 15, 0, 0]);
                            d
                        }
                        _ => vec![],
                    };
                    let event_packet = ZKPacket::new(CMD_REG_EVENT, session_id, 0, event_data);
                    stream
                        .write_all(&TCPWrapper::wrap(&event_packet.to_bytes()))
                        .unwrap();
                }
                CMD_OPTIONS_RRQ => {
                    let res = ZKPacket::new(
                        CMD_ACK_OK,
                        session_id,
                        packet.reply_id(),
                        b"TZAdj=7\0".to_vec(),
                    );
                    stream
                        .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                        .unwrap();
                }
                CMD_ACK_OK => {
                    // Client acknowledged the event
                }
                CMD_EXIT => {
                    let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id(), vec![]);
                    stream
                        .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                        .unwrap();
                    break;
                }
                _ => {
                    let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id(), vec![]);
                    stream
                        .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                        .unwrap();
                }
            }
        }
    });
}

#[test]
fn test_live_events_10byte_mock() {
    let _ = env_logger::builder().is_test(true).try_init();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();

    start_live_mock_server(listener, 1234, "10byte");

    let mut zk = ZK::new("127.0.0.1", addr.port());
    zk.set_timeout(Duration::from_millis(500));
    zk.connect(ZKProtocol::TCP).unwrap();

    let mut event_iter = zk.listen_events().unwrap();
    let event = event_iter.next().unwrap().unwrap();

    assert_eq!(event.uid(), 101);
    assert_eq!(event.user_id(), "101");
    assert_eq!(event.timestamp().to_string(), "2026-02-20 10:30:00");
}

#[test]
fn test_live_events_32byte_mock() {
    let _ = env_logger::builder().is_test(true).try_init();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();

    start_live_mock_server(listener, 4321, "32byte");

    let mut zk = ZK::new("127.0.0.1", addr.port());
    zk.set_timeout(Duration::from_millis(500));
    zk.connect(ZKProtocol::TCP).unwrap();

    let mut event_iter = zk.listen_events().unwrap();
    let event = event_iter.next().unwrap().unwrap();

    assert_eq!(event.user_id(), "RUST-USER-001");
    assert_eq!(event.status(), 1);
    assert_eq!(event.timestamp().to_string(), "2026-02-20 15:00:00");
}

fn start_live_mock_server_delayed(listener: TcpListener, session_id: u16) {
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();

        loop {
            let mut header = [0u8; 8];
            if stream.read_exact(&mut header).is_err() {
                break;
            }
            let (length, _) = TCPWrapper::decode_header(&header).unwrap();
            let mut body = vec![0u8; length];
            stream.read_exact(&mut body).unwrap();
            let packet = ZKPacket::from_bytes_owned(body).unwrap();

            match packet.command() {
                CMD_CONNECT => {
                    let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id(), vec![]);
                    stream
                        .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                        .unwrap();
                }
                CMD_REG_EVENT => {
                    let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id(), vec![]);
                    stream
                        .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                        .unwrap();

                    // Sleep 350ms to guarantee multiple client-side read timeouts (client timeout is 100ms)
                    thread::sleep(Duration::from_millis(350));

                    let mut d = vec![0u8; 32];
                    d[..13].copy_from_slice(b"RUST-USER-001");
                    d[24] = 1;
                    d[25] = 0;
                    d[26..32].copy_from_slice(&[26, 2, 20, 15, 0, 0]);
                    let event_packet = ZKPacket::new(CMD_REG_EVENT, session_id, 0, d);
                    stream
                        .write_all(&TCPWrapper::wrap(&event_packet.to_bytes()))
                        .unwrap();
                }
                CMD_ACK_OK => {}
                CMD_EXIT => {
                    let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id(), vec![]);
                    stream
                        .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                        .unwrap();
                    break;
                }
                _ => {
                    let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id(), vec![]);
                    stream
                        .write_all(&TCPWrapper::wrap(&res.to_bytes()))
                        .unwrap();
                }
            }
        }
    });
}

#[test]
fn test_live_events_timeout_retry_mock() {
    let _ = env_logger::builder().is_test(true).try_init();
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind");
    let addr = listener.local_addr().unwrap();
    let port = addr.port();

    start_live_mock_server_delayed(listener, 4321);

    let mut zk = ZK::new("127.0.0.1", port);
    // Set a very short timeout (100ms) so multiple timeouts occur during the 350ms delay
    zk.set_timeout(Duration::from_millis(100));
    zk.connect(ZKProtocol::TCP).unwrap();

    let mut event_iter = zk.listen_events().unwrap();
    // The call to next() will block, timeout several times, loop internally,
    // and finally yield the event when it arrives at ~350ms.
    let event = event_iter.next().unwrap().unwrap();

    assert_eq!(event.user_id(), "RUST-USER-001");
    assert_eq!(event.status(), 1);
    assert_eq!(event.timestamp().to_string(), "2026-02-20 15:00:00");

    drop(event_iter);
    zk.disconnect().unwrap();
}
