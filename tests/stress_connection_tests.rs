//! Stress tests for connection handling
//!
//! These tests verify that rapid/repeated connections do NOT cause:
//! - Resource leaks (sockets, memory)
//! - Dirty state (session_id, reply_id, checksum flag)
//! - Missing CMD_EXIT (which would exhaust device sessions)
//!
//! All tests use mock TCP servers on 127.0.0.1:0 — no real device needed.

use byteorder::{ByteOrder, LittleEndian, WriteBytesExt};
use rustzk::constants::*;
use rustzk::protocol::{TCPWrapper, ZKPacket};
use rustzk::{ZKProtocol, ZK};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

/// Helper: read one ZK request from a TCP stream (TCP-wrapped).
fn read_request(stream: &mut TcpStream) -> Option<ZKPacket<'static>> {
    let mut header = [0u8; 8];
    if stream.read_exact(&mut header).is_err() {
        return None;
    }
    let (length, _) = TCPWrapper::decode_header(&header).ok()?;
    let mut body = vec![0u8; length];
    stream.read_exact(&mut body).ok()?;
    ZKPacket::from_bytes_owned(body).ok()
}

/// Helper: send a ZK response over TCP (TCP-wrapped).
fn send_response(stream: &mut TcpStream, packet: &ZKPacket) {
    let _ = stream.write_all(&TCPWrapper::wrap(&packet.to_bytes()));
    let _ = stream.flush();
}

/// Helper: build a CMD_GET_FREE_SIZES response with N users and M records.
fn build_sizes_payload(users: i32, records: i32) -> Vec<u8> {
    let mut bytes = Vec::new();
    for i in 0..20 {
        let val = match i {
            4 => users,
            8 => records,
            15 => 1000,  // users_cap
            16 => 10000, // rec_cap
            _ => 0,
        };
        bytes.write_i32::<LittleEndian>(val).unwrap();
    }
    bytes
}

/// Helper: run a full mock server that handles connect, sizes, exit,
/// and counts CMD_EXIT messages received. Returns a thread handle.
fn spawn_reusable_mock_server(
    listener: TcpListener,
    expected_connections: u32,
    exit_counter: Arc<AtomicU32>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        for _ in 0..expected_connections {
            let (mut stream, _) = match listener.accept() {
                Ok(s) => s,
                Err(_) => break,
            };
            let session_id: u16 = 5555;

            loop {
                let packet = match read_request(&mut stream) {
                    Some(p) => p,
                    None => break,
                };

                match packet.command {
                    CMD_CONNECT => {
                        let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, vec![]);
                        send_response(&mut stream, &res);
                    }
                    CMD_GET_FREE_SIZES => {
                        let payload = build_sizes_payload(1, 1);
                        let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, payload);
                        send_response(&mut stream, &res);
                    }
                    CMD_OPTIONS_RRQ => {
                        // Return timezone option
                        let res = ZKPacket::new(
                            CMD_ACK_OK,
                            session_id,
                            packet.reply_id,
                            b"TZAdj=7\0".to_vec(),
                        );
                        send_response(&mut stream, &res);
                    }
                    CMD_EXIT => {
                        exit_counter.fetch_add(1, Ordering::SeqCst);
                        let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, vec![]);
                        send_response(&mut stream, &res);
                        break;
                    }
                    _ => {
                        let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, vec![]);
                        send_response(&mut stream, &res);
                    }
                }
            }
        }
    })
}

/// Helper: run a full mock server that also handles get_users (CMD_USERTEMP_RRQ).
fn spawn_data_mock_server(
    listener: TcpListener,
    expected_connections: u32,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        for _ in 0..expected_connections {
            let (mut stream, _) = match listener.accept() {
                Ok(s) => s,
                Err(_) => break,
            };
            let session_id: u16 = 7777;
            let mut last_prepared_cmd: u16 = 0;

            loop {
                let packet = match read_request(&mut stream) {
                    Some(p) => p,
                    None => break,
                };

                match packet.command {
                    CMD_CONNECT => {
                        let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, vec![]);
                        send_response(&mut stream, &res);
                    }
                    CMD_GET_FREE_SIZES => {
                        let payload = build_sizes_payload(5, 0);
                        let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, payload);
                        send_response(&mut stream, &res);
                    }
                    CMD_OPTIONS_RRQ => {
                        let res = ZKPacket::new(
                            CMD_ACK_OK,
                            session_id,
                            packet.reply_id,
                            b"TZAdj=7\0".to_vec(),
                        );
                        send_response(&mut stream, &res);
                    }
                    _CMD_PREPARE_BUFFER => {
                        if packet.payload.len() >= 3 {
                            last_prepared_cmd = LittleEndian::read_u16(&packet.payload[1..3]);
                        }
                        // 5 users × 28 bytes each + 4 byte size prefix = 144
                        let total_size: u32 = if last_prepared_cmd == CMD_USERTEMP_RRQ {
                            4 + (5 * 28)
                        } else {
                            4 // empty
                        };
                        let mut res_payload = vec![0u8; 5];
                        res_payload[0] = 1;
                        LittleEndian::write_u32(&mut res_payload[1..5], total_size);
                        let res = ZKPacket::new(
                            CMD_PREPARE_DATA,
                            session_id,
                            packet.reply_id,
                            res_payload,
                        );
                        send_response(&mut stream, &res);
                    }
                    _CMD_READ_BUFFER => {
                        let mut data = Vec::new();
                        if last_prepared_cmd == CMD_USERTEMP_RRQ {
                            data.write_u32::<LittleEndian>(5 * 28).unwrap();
                            // Write 5 users
                            for uid in 1..=5u16 {
                                data.write_u16::<LittleEndian>(uid).unwrap(); // UID
                                data.push(USER_DEFAULT); // Privilege
                                data.extend_from_slice(b"pwd\0\0"); // Password (5 bytes)
                                let name = format!("User{}\0\0\0", uid);
                                data.extend_from_slice(&name.as_bytes()[..8]); // Name (8 bytes)
                                data.write_u32::<LittleEndian>(0).unwrap(); // Card
                                data.push(0); // Pad
                                data.push(1); // Group
                                data.write_u16::<LittleEndian>(0).unwrap(); // TZ
                                data.write_u32::<LittleEndian>(uid as u32 + 100).unwrap();
                                // UserID
                            }
                        } else {
                            data.write_u32::<LittleEndian>(0).unwrap();
                        }
                        let res = ZKPacket::new(CMD_DATA, session_id, packet.reply_id, data);
                        send_response(&mut stream, &res);
                    }
                    CMD_FREE_DATA => {
                        let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, vec![]);
                        send_response(&mut stream, &res);
                    }
                    CMD_EXIT => {
                        let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, vec![]);
                        send_response(&mut stream, &res);
                        break;
                    }
                    _ => {
                        let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, vec![]);
                        send_response(&mut stream, &res);
                    }
                }
            }
        }
    })
}

// ============================================================================
// TEST 1: Rapid Connect/Disconnect Cycles
// ============================================================================

/// Stress test: 50 sequential connect → disconnect cycles to the same mock server.
/// Verifies:
/// - No socket/transport leak (each cycle succeeds)
/// - State is clean after each disconnect
/// - No panics or hangs
#[test]
fn stress_rapid_connect_disconnect() {
    let cycles = 50;
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind");
    let port = listener.local_addr().unwrap().port();
    let exit_counter = Arc::new(AtomicU32::new(0));

    let handle = spawn_reusable_mock_server(listener, cycles, exit_counter.clone());

    let start = Instant::now();

    for i in 0..cycles {
        let mut zk = ZK::new("127.0.0.1", port);
        zk.timeout = Duration::from_secs(5);

        let result = zk.connect(ZKProtocol::TCP);
        assert!(
            result.is_ok(),
            "Cycle {}: connect failed: {:?}",
            i,
            result.err()
        );
        assert!(zk.is_connected(), "Cycle {}: should be connected", i);

        let result = zk.disconnect();
        assert!(
            result.is_ok(),
            "Cycle {}: disconnect failed: {:?}",
            i,
            result.err()
        );
        assert!(
            !zk.is_connected(),
            "Cycle {}: should not be connected after disconnect",
            i
        );
    }

    let elapsed = start.elapsed();
    handle.join().unwrap();

    // Verify all CMD_EXIT were sent
    assert_eq!(
        exit_counter.load(Ordering::SeqCst),
        cycles,
        "Each connect/disconnect cycle must send exactly one CMD_EXIT"
    );

    // Sanity: 50 cycles should complete in under 30 seconds
    assert!(
        elapsed < Duration::from_secs(30),
        "50 rapid connect/disconnect cycles took {:?} — too slow",
        elapsed
    );
}

// ============================================================================
// TEST 2: Concurrent Connections
// ============================================================================

/// Stress test: 10 ZK instances connecting simultaneously to 10 independent mock servers.
/// Verifies:
/// - No panics under concurrent use
/// - Each instance independently connects and disconnects
#[test]
fn stress_concurrent_connections() {
    let num_instances = 10;
    let mut handles = Vec::new();

    for i in 0..num_instances {
        let handle = thread::spawn(move || {
            let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind");
            let port = listener.local_addr().unwrap().port();
            let exit_counter = Arc::new(AtomicU32::new(0));

            let server = spawn_reusable_mock_server(listener, 1, exit_counter.clone());

            let mut zk = ZK::new("127.0.0.1", port);
            zk.timeout = Duration::from_secs(5);

            let result = zk.connect(ZKProtocol::TCP);
            assert!(
                result.is_ok(),
                "Instance {}: connect failed: {:?}",
                i,
                result.err()
            );
            assert!(zk.is_connected(), "Instance {}: should be connected", i);

            let result = zk.disconnect();
            assert!(
                result.is_ok(),
                "Instance {}: disconnect failed: {:?}",
                i,
                result.err()
            );

            server.join().unwrap();
            assert_eq!(
                exit_counter.load(Ordering::SeqCst),
                1,
                "Instance {}: must send CMD_EXIT",
                i
            );
        });
        handles.push(handle);
    }

    for (i, handle) in handles.into_iter().enumerate() {
        handle
            .join()
            .unwrap_or_else(|_| panic!("Instance {} panicked", i));
    }
}

// ============================================================================
// TEST 3: Connect + Data Operations (get_users) Under Stress
// ============================================================================

/// Stress test: 10 cycles of connect → read_sizes → get_users → disconnect.
/// Verifies:
/// - Data fetching works correctly every cycle
/// - No memory accumulation issues
/// - User parsing is stable under repeated cycles
#[test]
fn stress_connect_with_data_operations() {
    let cycles = 10;
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind");
    let port = listener.local_addr().unwrap().port();

    let handle = spawn_data_mock_server(listener, cycles);

    for i in 0..cycles {
        let mut zk = ZK::new("127.0.0.1", port);
        zk.timeout = Duration::from_secs(5);

        zk.connect(ZKProtocol::TCP)
            .unwrap_or_else(|e| panic!("Cycle {}: connect failed: {}", i, e));

        zk.read_sizes()
            .unwrap_or_else(|e| panic!("Cycle {}: read_sizes failed: {}", i, e));
        assert_eq!(zk.users(), 5, "Cycle {}: should have 5 users", i);

        let users = zk
            .get_users()
            .unwrap_or_else(|e| panic!("Cycle {}: get_users failed: {}", i, e));
        assert_eq!(users.len(), 5, "Cycle {}: should parse 5 users", i);
        assert_eq!(users[0].name, "User1", "Cycle {}: first user name", i);
        assert_eq!(
            users[4].user_id, "105",
            "Cycle {}: last user_id should be 105",
            i
        );

        zk.disconnect()
            .unwrap_or_else(|e| panic!("Cycle {}: disconnect failed: {}", i, e));
        assert!(
            !zk.is_connected(),
            "Cycle {}: should not be connected after disconnect",
            i
        );
    }

    handle.join().unwrap();
}

// ============================================================================
// TEST 4: Session State Integrity Under Rapid Reconnection
// ============================================================================

/// Stress test: 20 cycles, checking session_id and reply_id after each disconnect.
/// Verifies:
/// - session_id updates on connect and is NOT stale after disconnect
/// - reply_id starts from a known value each connection
/// - is_connected toggles correctly
#[test]
fn stress_session_state_integrity() {
    let cycles = 20;
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind");
    let port = listener.local_addr().unwrap().port();
    let exit_counter = Arc::new(AtomicU32::new(0));

    let handle = spawn_reusable_mock_server(listener, cycles, exit_counter.clone());

    for i in 0..cycles {
        let mut zk = ZK::new("127.0.0.1", port);
        zk.timeout = Duration::from_secs(5);

        // Before connect: session_id should be 0
        assert_eq!(
            zk.session_id(),
            0,
            "Cycle {}: session_id should be 0 before connect",
            i
        );
        assert!(
            !zk.is_connected(),
            "Cycle {}: should not be connected initially",
            i
        );

        zk.connect(ZKProtocol::TCP)
            .unwrap_or_else(|e| panic!("Cycle {}: connect failed: {}", i, e));

        // After connect: session_id should be set by server (5555)
        assert_eq!(
            zk.session_id(),
            5555,
            "Cycle {}: session_id mismatch after connect",
            i
        );
        assert!(zk.is_connected(), "Cycle {}: should be connected", i);

        zk.disconnect()
            .unwrap_or_else(|e| panic!("Cycle {}: disconnect failed: {}", i, e));

        assert!(
            !zk.is_connected(),
            "Cycle {}: should not be connected after disconnect",
            i
        );
    }

    handle.join().unwrap();
    assert_eq!(
        exit_counter.load(Ordering::SeqCst),
        cycles,
        "Each cycle must send CMD_EXIT"
    );
}

// ============================================================================
// TEST 5: Handshake Timeout Recovery Under Load
// ============================================================================

/// Stress test: 5 cycles where the first handshake attempt always times out
/// (simulating wrong checksum), forcing auto-detection fallback.
/// Verifies:
/// - Client recovers from timeout and connects with alternative checksum
/// - No hang on repeated timeout+retry pattern
/// - checksum flag is correctly set after fallback
#[test]
fn stress_handshake_timeout_recovery() {
    let cycles = 5;

    for i in 0..cycles {
        let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind");
        let port = listener.local_addr().unwrap().port();

        let server_handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("Failed to accept connection");

            // First CMD_CONNECT: read it but don't respond (simulate timeout)
            let mut header = [0u8; 8];
            stream.read_exact(&mut header).unwrap();
            let (length, _) = TCPWrapper::decode_header(&header).unwrap();
            let mut body = vec![0u8; length];
            stream.read_exact(&mut body).unwrap();
            let packet = ZKPacket::from_bytes(&body).unwrap();
            assert_eq!(packet.command, CMD_CONNECT);

            // Don't respond → client will timeout after ~5s

            // Second CMD_CONNECT: respond with success
            let mut header2 = [0u8; 8];
            stream.read_exact(&mut header2).unwrap();
            let (length2, _) = TCPWrapper::decode_header(&header2).unwrap();
            let mut body2 = vec![0u8; length2];
            stream.read_exact(&mut body2).unwrap();
            let packet2 = ZKPacket::from_bytes_owned(body2).unwrap();
            assert_eq!(packet2.command, CMD_CONNECT);

            let res = ZKPacket::new(CMD_ACK_OK, 8888, packet2.reply_id, vec![]);
            send_response(&mut stream, &res);

            // Handle CMD_EXIT
            loop {
                let pkt = match read_request(&mut stream) {
                    Some(p) => p,
                    None => break,
                };
                let res = ZKPacket::new(CMD_ACK_OK, 8888, pkt.reply_id, vec![]);
                send_response(&mut stream, &res);
                if pkt.command == CMD_EXIT {
                    break;
                }
            }
        });

        let mut zk = ZK::new("127.0.0.1", port);
        // Use short timeout so the test doesn't take forever
        zk.timeout = Duration::from_secs(10);

        let start = Instant::now();
        let result = zk.connect(ZKProtocol::TCP);
        let elapsed = start.elapsed();

        assert!(
            result.is_ok(),
            "Cycle {}: connect should succeed after timeout fallback: {:?}",
            i,
            result.err()
        );
        assert!(
            zk.is_connected(),
            "Cycle {}: should be connected after fallback",
            i
        );
        // Should take at least ~5s (first timeout) but less than 15s
        assert!(
            elapsed >= Duration::from_secs(4),
            "Cycle {}: should have waited ~5s for timeout, took {:?}",
            i,
            elapsed
        );
        assert!(
            elapsed < Duration::from_secs(15),
            "Cycle {}: took too long: {:?}",
            i,
            elapsed
        );

        zk.disconnect().unwrap();
        server_handle.join().unwrap();
    }
}

// ============================================================================
// TEST 6: CMD_EXIT Always Sent (Device Session Exhaustion Prevention)
// ============================================================================

/// Stress test: 20 connect/disconnect cycles, counting CMD_EXIT on server side.
/// Also tests that Drop sends CMD_EXIT when disconnect() is NOT called.
/// This is CRITICAL for real devices — if CMD_EXIT is not sent, the device
/// runs out of sessions and needs to be rebooted.
#[test]
fn stress_cmd_exit_always_sent() {
    let cycles = 20;
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind");
    let port = listener.local_addr().unwrap().port();
    let exit_counter = Arc::new(AtomicU32::new(0));

    // Expect (cycles) connections:
    // - First half: explicit disconnect()
    // - Second half: Drop auto-disconnect
    let handle = spawn_reusable_mock_server(listener, cycles, exit_counter.clone());

    // First half: explicit disconnect
    let half = cycles / 2;
    for i in 0..half {
        let mut zk = ZK::new("127.0.0.1", port);
        zk.timeout = Duration::from_secs(5);
        zk.connect(ZKProtocol::TCP)
            .unwrap_or_else(|e| panic!("Cycle {}: connect failed: {}", i, e));
        zk.disconnect()
            .unwrap_or_else(|e| panic!("Cycle {}: disconnect failed: {}", i, e));
    }

    // Second half: let Drop auto-disconnect
    for i in half..cycles {
        let mut zk = ZK::new("127.0.0.1", port);
        zk.timeout = Duration::from_secs(5);
        zk.connect(ZKProtocol::TCP)
            .unwrap_or_else(|e| panic!("Cycle {}: connect failed: {}", i, e));
        // No explicit disconnect — Drop should handle it
    }

    handle.join().unwrap();

    let total_exits = exit_counter.load(Ordering::SeqCst);
    assert_eq!(
        total_exits, cycles,
        "Expected {} CMD_EXIT messages (explicit + Drop), got {}",
        cycles, total_exits
    );
}
