use byteorder::{ByteOrder, LittleEndian, WriteBytesExt};
use rustzk::{ZKProtocol, ZK};
use rustzk::constants::*;
use std::env;
use std::io::{Read, Write};

fn main() {
    let args: Vec<String> = env::args().collect();
    let (ip, port) = if args.len() >= 3 {
        (args[1].clone(), args[2].parse().unwrap_or(4370))
    } else {
        ("192.168.12.14".to_string(), 4370)
    };

    println!("Connecting to {}:{}...", ip, port);
    let mut zk = ZK::new(&ip, port);
    zk.connect(ZKProtocol::Auto).unwrap();

    let attempts = [
        ("CMD_USERTEMP_RRQ (9), FCT_USER (5), PREPARE=1503", CMD_USERTEMP_RRQ, 5, 1503),
        ("CMD_USERTEMP_RRQ (9), FCT_USER (5), PREPARE=1500", CMD_USERTEMP_RRQ, 5, 1500),
        ("CMD_DB_RRQ (7), FCT_USER (5), PREPARE=1503", CMD_DB_RRQ, 5, 1503),
        ("CMD_DB_RRQ (7), FCT_USER (5), PREPARE=1500", CMD_DB_RRQ, 5, 1500),
        ("CMD_USER_WRQ (8), FCT_USER (5), PREPARE=1503", CMD_USER_WRQ, 5, 1503),
    ];

    for (desc, cmd, fct, prepare_cmd) in attempts.iter() {
        println!("\n--- Testing {} ---", desc);
        
        // Manual read_with_buffer logic
        let mut payload = Vec::new();
        payload.write_u8(1).unwrap();
        payload.write_u16::<LittleEndian>(*cmd).unwrap();
        payload.write_u8(*fct).unwrap();
        payload.write_u32::<LittleEndian>(0).unwrap(); // ext

        match zk.send_command(*prepare_cmd, payload) {
            Ok(res) => {
                println!("Response cmd={}, payload_len={}", res.command, res.payload.len());
                if res.command == CMD_DATA {
                    println!("Success! data len={}", res.payload.len());
                } else if res.command == CMD_ACK_OK {
                     println!("Got ACK_OK. Maybe need to read chunks?");
                } else {
                    println!("Got other response: {}", res.command);
                }
            }
            Err(e) => println!("Error: {}", e),
        }
    }

    zk.disconnect().unwrap();
}
