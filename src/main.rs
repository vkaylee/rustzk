use rustzk::{ZKProtocol, ZK};
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <ip> [port]", args[0]);
        std::process::exit(1);
    }
    let ip = args[1].clone();
    let port = if args.len() > 2 {
        args[2].parse().unwrap_or(4370)
    } else {
        4370
    };

    println!("Connecting to ZK device at {}:{}...", ip, port);

    let mut zk = ZK::new(&ip, port);

    // Try to connect (Auto mode: TCP first, then UDP)
    if let Err(e) = zk.connect(ZKProtocol::Auto) {
        eprintln!("Failed to connect via UDP: {}", e);
        std::process::exit(1);
    }

    println!("Connected successfully! (Protocol: Auto)\n");

    if let Ok(sn) = zk.get_serial_number() {
        println!("Serial Number: {}", sn);
    }
    if let Ok(ver) = zk.get_firmware_version() {
        println!("Firmware: {}", ver);
    }
    if let Ok(plat) = zk.get_platform() {
        println!("Platform: {}", plat);
    }
    if let Ok(mac) = zk.get_mac() {
        println!("MAC: {}", mac);
    }
    if let Ok(name) = zk.get_device_name() {
        println!("Device Name: {}", name);
    }
    if let Ok(time) = zk.get_time() {
        println!("Device Time: {}", time);
    }

    println!("\n--- Device Capacity ---");
    if zk.read_sizes().is_ok() {
        println!("Users: {} / {}", zk.users(), zk.users_cap());
        println!("Fingers: {} / {}", zk.fingers(), zk.fingers_cap());
        println!(
            "Attendance Records: {} / {}",
            zk.records(),
            zk.records_cap()
        );
        if zk.faces_cap() > 0 {
            println!("Faces: {} / {}", zk.faces(), zk.faces_cap());
        }
    } else {
        println!("Failed to read sizes");
    }

    println!("\n--- User List ---");
    match zk.get_users() {
        Ok(users) => {
            println!("Total users found: {}", users.len());
            for user in &users {
                println!(
                    "  UID: {:>4} | ID: {:>10} | Name: {:>20} | Privilege: {} | Card: {}",
                    user.uid, user.user_id, user.name, user.privilege, user.card
                );
            }
        }
        Err(e) => println!("Failed to get users: {}", e),
    }

    let _ = zk.disconnect();
}
