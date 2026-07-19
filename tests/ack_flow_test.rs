use byteorder::{ByteOrder, LittleEndian, WriteBytesExt};
use rustzk::constants::*;
use rustzk::protocol::{TCPWrapper, ZKPacket};
use rustzk::{ZKProtocol, ZK};
use std::thread;
use std::time::Duration;

mod common;
use common::{send_response, MockZKServer};

#[test]
fn test_read_chunk_waits_for_data_after_ack() {
    let session_id: u16 = 9999;
    let (server, port) = MockZKServer::new()
        .with_session(session_id)
        .on(CMD_GET_FREE_SIZES, |_rid, _| {
            let mut bytes = Vec::new();
            for i in 0..20 {
                // idx 4=users, 8=records. Set 1 user, 1 record.
                let val = if i == 4 || i == 8 { 1 } else { 0 };
                bytes.write_i32::<LittleEndian>(val).unwrap();
            }
            Some((CMD_ACK_OK, bytes))
        })
        .on(_CMD_PREPARE_BUFFER, |_rid, _| {
            let size = 4 + 28;
            let mut res_payload = vec![0u8; 5];
            res_payload[0] = 1;
            LittleEndian::write_u32(&mut res_payload[1..5], size as u32);
            Some((CMD_PREPARE_DATA, res_payload))
        })
        .on_multi(_CMD_READ_BUFFER, move |rid, _payload, stream| {
            // 1. Send ACK_OK first
            let ack = ZKPacket::new(CMD_ACK_OK, session_id, rid, vec![]);
            send_response(stream, &ack);

            // Sleep to ensure client enters "wait for data" state
            thread::sleep(Duration::from_millis(10));

            // 2. Send CMD_DATA with user record
            let mut data = Vec::new();
            data.write_u32::<LittleEndian>(28).unwrap();
            data.write_u16::<LittleEndian>(1).unwrap();
            data.push(USER_DEFAULT);
            data.extend_from_slice(b"pwd\0\0");
            data.extend_from_slice(b"Test\0\0\0\0");
            data.write_u32::<LittleEndian>(0).unwrap();
            data.push(0);
            data.push(1);
            data.write_u16::<LittleEndian>(0).unwrap();
            data.write_u32::<LittleEndian>(101).unwrap();

            let res = ZKPacket::new(CMD_DATA, session_id, rid, data);
            send_response(stream, &res);
            false // don't send default ACK
        })
        .spawn();

    let mut zk = ZK::new("127.0.0.1", port);
    zk.connect(ZKProtocol::TCP).expect("Connect failed");

    let users = zk.get_users().expect("get_users failed");
    assert_eq!(users.len(), 1);
    assert_eq!(users[0].name(), "Test");

    zk.disconnect().unwrap();
    server.join();
}
