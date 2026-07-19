use byteorder::{ByteOrder, LittleEndian, WriteBytesExt};
use rustzk::constants::*;
use rustzk::{ZKProtocol, ZK};
use std::io::{Read, Write};

mod common;
use common::MockZKServer;

#[test]
fn test_get_option_and_timezone_mock() {
    let (server, port) = MockZKServer::new()
        .with_session(5566)
        .on(CMD_OPTIONS_RRQ, |_rid, payload| {
            let key = String::from_utf8_lossy(payload)
                .trim_matches('\0')
                .to_string();
            let response_str = match key.as_str() {
                "TZAdj" => "TZAdj=7\0",
                "~SerialNumber" => "~SerialNumber=ABC12345\0",
                _ => "Unknown=0\0",
            };
            Some((CMD_ACK_OK, response_str.as_bytes().to_vec()))
        })
        .spawn();

    let mut zk = ZK::new("127.0.0.1", port);
    zk.connect(ZKProtocol::Auto).unwrap();

    // Test get_option_value
    let sn = zk.get_option_value("~SerialNumber").unwrap();
    assert_eq!(sn, "ABC12345");

    // Test get_timezone
    let tz = zk.get_timezone().unwrap();
    assert_eq!(tz, 7);

    zk.disconnect().unwrap();
    server.join();
}

#[test]
fn test_attendance_heuristic_40bytes_mock() {
    let (server, port) = MockZKServer::new()
        .with_session(7788)
        .on(CMD_GET_FREE_SIZES, |_rid, _| {
            let mut bytes = Vec::new();
            for i in 0..20 {
                let val = if i == 8 { 1 } else { 0 }; // Report 1 record
                bytes.write_i32::<LittleEndian>(val).unwrap();
            }
            Some((CMD_ACK_OK, bytes))
        })
        .on(_CMD_PREPARE_BUFFER, |_rid, _| {
            let mut res_payload = vec![0u8; 5];
            res_payload[0] = 1;
            // Total size = 4 (prefix) + 40 (one 40-byte record) = 44
            LittleEndian::write_u32(&mut res_payload[1..5], 44);
            Some((CMD_PREPARE_DATA, res_payload))
        })
        .on(_CMD_READ_BUFFER, |_rid, _| {
            let mut data = Vec::new();
            data.write_u32::<LittleEndian>(40).unwrap(); // Size prefix

            // 40-byte record
            data.write_u16::<LittleEndian>(10).unwrap(); // UID
            let mut user_id_bytes = [0u8; 24];
            user_id_bytes[0..3].copy_from_slice(b"101");
            data.write_all(&user_id_bytes).unwrap();
            data.push(1); // Status
            data.write_u32::<LittleEndian>(839845230).unwrap(); // Time
            data.push(0); // Punch
            data.write_all(&[0u8; 8]).unwrap(); // Padding to reach 40 bytes

            Some((CMD_DATA, data))
        })
        .spawn();

    let mut zk = ZK::new("127.0.0.1", port);
    zk.connect(ZKProtocol::TCP).unwrap();

    let logs = zk.get_attendance().unwrap();
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].user_id(), "101");
    assert_eq!(logs[0].uid(), 10);

    zk.disconnect().unwrap();
    server.join();
}
