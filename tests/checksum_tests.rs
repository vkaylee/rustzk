//! Tests for dual checksum algorithm correctness and auto-detection.
//!
//! ## Background
//!
//! The ZK protocol uses a 16-bit checksum to validate packets. Two algorithms exist:
//!
//! | Algorithm | Computation            | Result for sum=999 | Used by                    |
//! |-----------|------------------------|---------------------|----------------------------|
//! | Default   | `!(sum as i32)` + loop | 0xFC17 (64535)      | Python pyzk, newer firmware |
//! | Legacy    | `!(sum as u16)`        | 0xFC18 (64536)      | ZAM180_TFT (Ver 6.60)      |
//!
//! The difference is exactly 1 for all inputs. Some firmware silently drops packets
//! with the wrong checksum, causing connection timeouts.

use rustzk::constants::*;
use rustzk::protocol::{calculate_checksum, TCPWrapper, ZKPacket};
use rustzk::{ZKProtocol, ZK};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;
use std::time::{Duration, Instant};

// =============================================================================
// Unit tests: verify the two checksum algorithms produce different results
// =============================================================================

/// Verify that default and legacy checksums differ by exactly 1.
#[test]
fn test_default_vs_legacy_checksum_differ_by_one() {
    // CMD_CONNECT packet: command=1000, session=0, reply=65534, no payload
    let default = ZKPacket::new(1000, 0, 65534, vec![]);
    let legacy = ZKPacket::new_with_legacy(1000, 0, 65534, vec![]);

    // Default (Python-aligned): 0xFC17
    assert_eq!(
        default.checksum, 0xFC17,
        "Default checksum should be 0xFC17"
    );
    // Legacy (Rust bitwise NOT): 0xFC18
    assert_eq!(legacy.checksum, 0xFC18, "Legacy checksum should be 0xFC18");
    // Difference is exactly 1
    assert_eq!(
        legacy.checksum - default.checksum,
        1,
        "Legacy should be exactly 1 greater than default"
    );
}

/// Verify the property holds for various payload sizes and values.
#[test]
fn test_checksum_difference_is_always_one() {
    let test_cases: Vec<(u16, u16, u16, Vec<u8>)> = vec![
        // (command, session_id, reply_id, payload)
        (1000, 0, 65534, vec![]),                     // CMD_CONNECT, empty
        (11, 1234, 1, b"TZAdj=7\0".to_vec()),         // CMD_OPTIONS_RRQ
        (7, 5678, 100, vec![0xFF; 64]),               // CMD_DB_RRQ, max bytes
        (50, 0, 0, vec![0x01, 0x02, 0x03]),           // Odd-length payload
        (CMD_ACK_OK, 10000, 32767, vec![0xAA; 1024]), // Large payload
    ];

    for (cmd, sid, rid, payload) in test_cases {
        let default = ZKPacket::new(cmd, sid, rid, payload.clone());
        let legacy = ZKPacket::new_with_legacy(cmd, sid, rid, payload);

        assert_eq!(
            legacy.checksum.wrapping_sub(default.checksum),
            1,
            "Default={:#06X} Legacy={:#06X} for cmd={}, sid={}, rid={}",
            default.checksum,
            legacy.checksum,
            cmd,
            sid,
            rid
        );
    }
}

/// Verify that both constructors produce valid serializable packets.
#[test]
fn test_both_checksums_produce_valid_packets() {
    let default = ZKPacket::new(CMD_CONNECT, 0, 65534, vec![]);
    let legacy = ZKPacket::new_with_legacy(CMD_CONNECT, 0, 65534, vec![]);

    // Both should serialize/deserialize without error
    let default_bytes = default.to_bytes();
    let legacy_bytes = legacy.to_bytes();

    let decoded_default = ZKPacket::from_bytes(&default_bytes).unwrap();
    let decoded_legacy = ZKPacket::from_bytes(&legacy_bytes).unwrap();

    assert_eq!(decoded_default.command, CMD_CONNECT);
    assert_eq!(decoded_legacy.command, CMD_CONNECT);
    assert_eq!(decoded_default.checksum, 0xFC17);
    assert_eq!(decoded_legacy.checksum, 0xFC18);
}

/// Verify the standalone calculate_checksum function matches packet checksum.
#[test]
fn test_standalone_checksum_matches_packet() {
    let data = vec![0xE8, 0x03, 0x00, 0x00, 0x00, 0x00, 0xFE, 0xFF];
    let standalone = calculate_checksum(&data);
    let packet = ZKPacket::new(1000, 0, 65534, vec![]);

    assert_eq!(
        standalone, packet.checksum,
        "Standalone and packet checksum should match"
    );
}

// =============================================================================
// Integration tests: checksum auto-detection during handshake
// =============================================================================

/// A mock server that ONLY accepts packets with the legacy checksum.
/// Simulates firmware like ZAM180 that silently drops default-checksum packets.
#[test]
fn test_auto_fallback_to_legacy_checksum() {
    let _ = env_logger::builder().is_test(true).try_init();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(15)))
            .unwrap();
        let session_id: u16 = 4444;

        // Read first CMD_CONNECT attempt (default checksum — we reject it silently)
        let mut header = [0u8; 8];
        stream.read_exact(&mut header).unwrap();
        let (length, _) = TCPWrapper::decode_header(&header).unwrap();
        let mut body = vec![0u8; length];
        stream.read_exact(&mut body).unwrap();
        let packet1 = ZKPacket::from_bytes_owned(body).unwrap();
        assert_eq!(packet1.command, CMD_CONNECT);

        // Verify this is the DEFAULT checksum by comparing with independently computed value
        let expected_default =
            ZKPacket::new(CMD_CONNECT, packet1.session_id, packet1.reply_id, vec![]);
        assert_eq!(
            packet1.checksum, expected_default.checksum,
            "First attempt should use default checksum"
        );
        // Don't send any response — simulate firmware that rejected the checksum.

        // Read second CMD_CONNECT attempt (legacy checksum — we accept it)
        stream.read_exact(&mut header).unwrap();
        let (length, _) = TCPWrapper::decode_header(&header).unwrap();
        let mut body = vec![0u8; length];
        stream.read_exact(&mut body).unwrap();
        let packet2 = ZKPacket::from_bytes_owned(body).unwrap();
        assert_eq!(packet2.command, CMD_CONNECT);

        // Verify this is the LEGACY checksum
        let expected_legacy =
            ZKPacket::new_with_legacy(CMD_CONNECT, packet2.session_id, packet2.reply_id, vec![]);
        assert_eq!(
            packet2.checksum, expected_legacy.checksum,
            "Second attempt should use legacy checksum"
        );

        // Confirm the two checksums differ by exactly 1
        assert_eq!(packet2.checksum - packet1.checksum, 1);

        // Accept — send CMD_ACK_OK response
        let res = ZKPacket::new(CMD_ACK_OK, session_id, packet2.reply_id, vec![]);
        stream
            .write_all(&TCPWrapper::wrap(&res.to_bytes()))
            .unwrap();
        stream.flush().unwrap();
    });

    let mut zk = ZK::new("127.0.0.1", port);
    let start = Instant::now();
    zk.connect(ZKProtocol::TCP).unwrap();
    let elapsed = start.elapsed();

    assert!(zk.is_connected(), "Should be connected after auto-fallback");
    // Auto-detection should take ~5s (first timeout) + connect time
    assert!(
        elapsed >= Duration::from_secs(4),
        "Should have waited for first timeout: {:?}",
        elapsed
    );
    assert!(
        elapsed < Duration::from_secs(10),
        "Should not take too long: {:?}",
        elapsed
    );

    server.join().unwrap();
}

/// set_legacy_checksum(true) should connect immediately without auto-detection delay.
#[test]
fn test_set_legacy_checksum_skips_autodetect() {
    let _ = env_logger::builder().is_test(true).try_init();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let session_id: u16 = 5555;

        // Read CMD_CONNECT — should be legacy checksum immediately
        let mut header = [0u8; 8];
        stream.read_exact(&mut header).unwrap();
        let (length, _) = TCPWrapper::decode_header(&header).unwrap();
        let mut body = vec![0u8; length];
        stream.read_exact(&mut body).unwrap();
        let packet = ZKPacket::from_bytes_owned(body).unwrap();
        assert_eq!(packet.command, CMD_CONNECT);

        // Verify legacy checksum by comparing against independently computed value
        let expected_legacy =
            ZKPacket::new_with_legacy(CMD_CONNECT, packet.session_id, packet.reply_id, vec![]);
        assert_eq!(
            packet.checksum, expected_legacy.checksum,
            "With set_legacy_checksum(true), first packet should use legacy"
        );

        let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, vec![]);
        stream
            .write_all(&TCPWrapper::wrap(&res.to_bytes()))
            .unwrap();
        stream.flush().unwrap();
    });

    let mut zk = ZK::new("127.0.0.1", port);
    zk.set_legacy_checksum(true); // Force legacy — no auto-detect delay

    let start = Instant::now();
    zk.connect(ZKProtocol::TCP).unwrap();
    let elapsed = start.elapsed();

    assert!(zk.is_connected(), "Should connect with legacy checksum");
    // Should be near-instant (no 5s timeout for auto-detection)
    assert!(
        elapsed < Duration::from_secs(2),
        "Legacy mode should connect instantly: {:?}",
        elapsed
    );

    server.join().unwrap();
}

/// Verify that subsequent commands after auto-detected legacy keep using legacy.
#[test]
fn test_subsequent_commands_use_detected_checksum() {
    let _ = env_logger::builder().is_test(true).try_init();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(15)))
            .unwrap();
        let session_id: u16 = 6666;

        // 1. First CMD_CONNECT (default checksum) — silently drop
        let mut header = [0u8; 8];
        stream.read_exact(&mut header).unwrap();
        let (length, _) = TCPWrapper::decode_header(&header).unwrap();
        let mut body = vec![0u8; length];
        stream.read_exact(&mut body).unwrap();
        // Don't respond — simulates silent checksum rejection

        // 2. Second CMD_CONNECT (legacy checksum) — accept
        stream.read_exact(&mut header).unwrap();
        let (length, _) = TCPWrapper::decode_header(&header).unwrap();
        let mut body = vec![0u8; length];
        stream.read_exact(&mut body).unwrap();
        let packet = ZKPacket::from_bytes_owned(body).unwrap();

        // Verify legacy checksum
        let expected =
            ZKPacket::new_with_legacy(CMD_CONNECT, packet.session_id, packet.reply_id, vec![]);
        assert_eq!(packet.checksum, expected.checksum, "Should be legacy");

        let res = ZKPacket::new(CMD_ACK_OK, session_id, packet.reply_id, vec![]);
        stream
            .write_all(&TCPWrapper::wrap(&res.to_bytes()))
            .unwrap();

        // 3. Read next command (CMD_GET_VERSION) — should also use legacy checksum
        stream.read_exact(&mut header).unwrap();
        let (length, _) = TCPWrapper::decode_header(&header).unwrap();
        let mut body = vec![0u8; length];
        stream.read_exact(&mut body).unwrap();
        let packet = ZKPacket::from_bytes_owned(body).unwrap();
        assert_eq!(packet.command, CMD_GET_VERSION);

        // Verify the subsequent command ALSO uses legacy checksum
        let expected =
            ZKPacket::new_with_legacy(CMD_GET_VERSION, packet.session_id, packet.reply_id, vec![]);
        assert_eq!(
            packet.checksum, expected.checksum,
            "Subsequent commands should continue using legacy checksum"
        );

        // Respond with firmware version
        let res = ZKPacket::new(
            CMD_ACK_OK,
            session_id,
            packet.reply_id,
            b"Ver 6.60 Test\0".to_vec(),
        );
        stream
            .write_all(&TCPWrapper::wrap(&res.to_bytes()))
            .unwrap();
        stream.flush().unwrap();
    });

    let mut zk = ZK::new("127.0.0.1", port);
    zk.connect(ZKProtocol::TCP).unwrap();

    // This command should use the detected legacy checksum automatically
    let fw = zk.get_firmware_version().unwrap();
    assert_eq!(fw, "Ver 6.60 Test");

    server.join().unwrap();
}
