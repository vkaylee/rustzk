use rustzk::{ZKProtocol, ZK};
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let (ip, port) = if args.len() >= 3 {
        (args[1].clone(), args[2].parse().unwrap_or(4370))
    } else if args.len() == 2 {
        (args[1].clone(), 4370)
    } else {
        eprintln!("Usage: {} <ip> [port]", args[0]);
        eprintln!("Example: {} 192.168.12.14 4370", args[0]);
        std::process::exit(1);
    };

    println!("=== ZK Attendance Log Fetcher ===");
    println!("Connecting to {}:{}...", ip, port);

    let mut zk = ZK::new(&ip, port);

    if let Err(e) = zk.connect(ZKProtocol::Auto) {
        eprintln!("Failed to connect: {}", e);
        std::process::exit(1);
    }

    println!("Connected!\n");

    // Device info
    if let Ok(sn) = zk.get_serial_number() {
        println!("Serial Number : {}", sn);
    }
    if let Ok(fw) = zk.get_firmware_version() {
        println!("Firmware      : {}", fw);
    }
    if let Ok(plat) = zk.get_platform() {
        println!("Platform      : {}", plat);
    }
    if let Ok(t) = zk.get_time() {
        println!("Device Time   : {}", t.to_rfc3339());

        // Sync time if drift > 5 seconds
        let now = chrono::Local::now();
        let now_fixed = chrono::DateTime::<chrono::FixedOffset>::from(now);
        let drift = (now_fixed.timestamp() - t.timestamp()).abs();
        if drift > 5 {
            println!("Drift detected: {}s. Syncing time...", drift);
            if zk.set_time(now_fixed).is_ok() {
                println!("Time synced successfully.");
                if let Ok(new_t) = zk.get_time() {
                    println!("New Device Time: {}", new_t.to_rfc3339());
                }
            } else {
                println!("Failed to sync time.");
            }
        }
    }

    // Capacity
    println!("\n--- Capacity ---");
    if zk.read_sizes().is_ok() {
        println!("Users    : {} / {}", zk.users, zk.users_cap);
        println!("Records  : {} / {}", zk.records, zk.rec_cap);
        println!("Fingers  : {} / {}", zk.fingers, zk.fingers_cap);
        if zk.faces_cap > 0 {
            println!("Faces    : {} / {}", zk.faces, zk.faces_cap);
        }
    }

    // Fetch attendance
    println!("\n--- Fetching Attendance Logs ---");
    println!("(This may take a while for large datasets...)");

    match zk.get_attendance() {
        Ok(logs) => {
            if logs.is_empty() {
                println!("No attendance records found.");
            } else {
                println!("Total records: {}\n", logs.len());
                println!(
                    "{:<6} {:<15} {:<25} {:<8} {:<6}",
                    "UID", "User ID", "Timestamp (ISO-8601)", "Status", "Punch"
                );
                println!("{}", "-".repeat(65));

                for log in &logs {
                    println!(
                        "{:<6} {:<15} {:<25} {:<8} {:<6}",
                        log.uid,
                        log.user_id,
                        log.iso_format(),
                        log.status,
                        log.punch
                    );
                }

                // Stats
                println!("\n--- Summary ---");
                println!("Total attendance records : {}", logs.len());

                // Count unique users
                let mut user_ids: Vec<&str> = logs.iter().map(|l| l.user_id.as_str()).collect();
                user_ids.sort_unstable();
                user_ids.dedup();
                println!("Unique users (with logs) : {}", user_ids.len());

                // Date range
                if let (Some(first), Some(last)) = (logs.first(), logs.last()) {
                    println!("Earliest record          : {}", first.iso_format());
                    println!("Latest record            : {}", last.iso_format());
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to get attendance: {}", e);
            std::process::exit(1);
        }
    }

    let _ = zk.disconnect();
    println!("\nDone.");
}
