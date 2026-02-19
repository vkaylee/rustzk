use rustzk::constants::{CMD_ATTLOG_RRQ, CMD_DB_RRQ, FCT_ATTLOG};
use rustzk::{ZKProtocol, ZK};
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let (ip, port) = if args.len() >= 3 {
        (args[1].clone(), args[2].parse().unwrap_or(4370))
    } else {
        ("192.168.12.13".to_string(), 4370)
    };

    println!("Checking log count for {}:{}...", ip, port);

    let mut zk = ZK::new(&ip, port);

    if let Err(e) = zk.connect(ZKProtocol::Auto) {
        eprintln!("Failed to connect: {}", e);
        std::process::exit(1);
    }

    println!("Connected!");

    if let Err(e) = zk.read_sizes() {
        eprintln!("Failed to read sizes: {}", e);
    } else {
        println!("Records reported by device: {}", zk.records);
    }

    println!("\n--- Attempting to Disable Device ---");
    use rustzk::constants::{CMD_DISABLEDEVICE, CMD_ENABLEDEVICE, CMD_OPTIONS_WRQ};
    if let Err(e) = zk.send_command(CMD_DISABLEDEVICE, Vec::new()) {
        eprintln!("Failed to disable device: {}", e);
    } else {
        println!("Device disabled.");
    }

    println!("\n--- Setting SDKBuild=1 ---");
    if let Err(e) = zk.send_command(CMD_OPTIONS_WRQ, b"SDKBuild=1\0".to_vec()) {
        eprintln!("Failed to set SDKBuild=1: {}", e);
    } else {
        println!("SDKBuild=1 set.");
    }

    println!("\n--- Attempt 6: Custom get_attendance(fct=0, no get_users) ---");
    match custom_get_attendance(&mut zk, 0) {
        Ok(logs) => {
            println!("Custom get_attendance(0) returned {} logs", logs.len());
            if let Some(first) = logs.first() {
                println!(
                    "FIRST LOG: UserID={}, Time={}, Status={}, Punch={}",
                    first.user_id, first.timestamp, first.status, first.punch
                );
            }
            if let Some(last) = logs.last() {
                println!(
                    "LAST LOG:  UserID={}, Time={}, Status={}, Punch={}",
                    last.user_id, last.timestamp, last.status, last.punch
                );
            }
        }
        Err(e) => eprintln!("Custom get_attendance(0) failed: {}", e),
    }

    println!("\n--- Re-enabling Device ---");
    let _ = zk.send_command(CMD_ENABLEDEVICE, Vec::new());

    zk.disconnect().unwrap();
}

fn custom_get_attendance(
    zk: &mut ZK,
    fct: u8,
) -> rustzk::ZKResult<Vec<rustzk::models::Attendance>> {
    zk.read_sizes()?;
    if zk.records == 0 {
        return Ok(Vec::new());
    }

    // Skip get_users() for debugging
    let attendance_data = zk.read_with_buffer(CMD_ATTLOG_RRQ, fct, 0)?;
    println!(
        "DEBUG: Custom get_attendance received {} bytes raw buffer data",
        attendance_data.len()
    );
    if attendance_data.len() < 4 {
        return Ok(Vec::new());
    }

    let total_size = u32::from_le_bytes(attendance_data[0..4].try_into().unwrap()) as usize;
    println!("DEBUG: Total logs data size reported: {} bytes", total_size);
    let record_size = total_size / zk.records as usize;
    println!("DEBUG: Calculated record size: {} bytes", record_size);
    let data = &attendance_data[4..];

    let mut attendances = Vec::new();
    let mut offset = 0;

    if record_size == 8 {
        while offset + 8 <= data.len() {
            let chunk = &data[offset..offset + 8];
            let uid = u16::from_le_bytes(chunk[0..2].try_into().unwrap());
            let status = chunk[2];
            let time_bytes = &chunk[3..7];
            let punch = chunk[7];

            let timestamp = ZK::decode_time(time_bytes)?;
            attendances.push(rustzk::models::Attendance {
                uid: uid as u32,
                user_id: uid.to_string(), // Dummy user_id
                timestamp,
                status,
                punch,
            });
            offset += 8;
        }
    } else if record_size == 16 {
        while offset + 16 <= data.len() {
            let chunk = &data[offset..offset + 16];
            let user_id_num = u32::from_le_bytes(chunk[0..4].try_into().unwrap());
            let time_bytes = &chunk[4..8];
            let status = chunk[8];
            let punch = chunk[9];

            let timestamp = ZK::decode_time(time_bytes)?;
            attendances.push(rustzk::models::Attendance {
                uid: user_id_num,
                user_id: user_id_num.to_string(),
                timestamp,
                status,
                punch,
            });
            offset += 16;
        }
    } else if record_size >= 40 {
        while offset + 40 <= data.len() {
            let chunk = &data[offset..offset + 40];
            let mut chunk_ptr = chunk;
            if chunk.starts_with(b"\xff255\x00\x00\x00\x00\x00") {
                chunk_ptr = &chunk[10..];
            }

            let uid = u16::from_le_bytes(chunk_ptr[0..2].try_into().unwrap());
            let user_id = String::from_utf8_lossy(&chunk_ptr[2..26])
                .trim_matches('\0')
                .to_string();
            let status = chunk_ptr[26];
            let time_bytes = &chunk_ptr[27..31];
            let punch = chunk_ptr[31];

            let timestamp = ZK::decode_time(time_bytes)?;
            attendances.push(rustzk::models::Attendance {
                uid: uid as u32,
                user_id,
                timestamp,
                status,
                punch,
            });
            offset += record_size;
        }
    }

    Ok(attendances)
}
