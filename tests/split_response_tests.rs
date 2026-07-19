use byteorder::{ByteOrder, LittleEndian};
use rustzk::constants::*;
use rustzk::protocol::{TCPWrapper, ZKPacket};
use rustzk::{ZKProtocol, ZK};

mod common;
use common::{send_response, MockZKServer};

#[test]
fn test_read_sizes_split_response_tcp() {
    let _ = env_logger::builder().is_test(true).try_init();

    let session_id: u16 = 5678;
    let (server, port) = MockZKServer::new()
        .with_session(session_id)
        .on_multi(CMD_GET_FREE_SIZES, move |rid, _payload, stream| {
            // Send empty ACK_OK first (simulating split response)
            let ack_ok = ZKPacket::new(CMD_ACK_OK, session_id, rid, vec![]);
            send_response(stream, &ack_ok);

            // Then send ACK_DATA with actual sizes payload
            let mut sizes_payload = vec![0u8; 92];
            LittleEndian::write_i32(&mut sizes_payload[16..20], 123); // users
            LittleEndian::write_i32(&mut sizes_payload[32..36], 456); // records

            let data_res = ZKPacket::new(CMD_ACK_DATA, session_id, rid, sizes_payload);
            send_response(stream, &data_res);
            false // don't send default ACK
        })
        .on(CMD_OPTIONS_RRQ, |_rid, _| {
            Some((CMD_ACK_OK, b"TZAdj=7\0".to_vec()))
        })
        .spawn();

    let mut zk = ZK::new("127.0.0.1", port);
    zk.connect(ZKProtocol::TCP).unwrap();

    // Trigger read_sizes
    zk.read_sizes()
        .expect("read_sizes should handle split response");

    // Verify fields were populated from the SECOND packet
    assert_eq!(zk.users(), 123);
    assert_eq!(zk.records(), 456);

    server.join();
}
