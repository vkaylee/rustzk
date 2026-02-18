use rustzk::{ZKProtocol, ZK};
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let (ip, port) = if args.len() >= 3 {
        (args[1].clone(), args[2].parse().unwrap_or(4370))
    } else {
        ("192.168.12.14".to_string(), 4370)
    };

    println!("Connecting to ZK device at {}:{}...", ip, port);

    let mut zk = ZK::new(&ip, port);

    // Try to connect (TCP)
    if let Err(e) = zk.connect(ZKProtocol::Auto) {
        eprintln!("Failed to connect: {}", e);
        std::process::exit(1);
    }

    println!("Connected successfully!\n");

    // Device info
    if let Ok(sn) = zk.get_serial_number() {
        println!("Serial Number: {}", sn);
    }
    if let Ok(name) = zk.get_device_name() {
        println!("Device Name: {}", name);
    }

    // Read sizes to get user count
    println!("\n--- Device Capacity ---");
    match zk.read_sizes() {
        Ok(_) => {
            // Read sizes to get user count
            println!("\n--- Device Capacity ---");
            match zk.read_sizes() {
                Ok(_) => {
                    println!("Users: {} / {}", zk.users, zk.users_cap);
                    println!("Fingers: {} / {}", zk.fingers, zk.fingers_cap);
                    println!("Records: {} / {}", zk.records, zk.rec_cap);
                    if zk.faces_cap > 0 {
                        println!("Faces: {} / {}", zk.faces, zk.faces_cap);
                    }
                }
                Err(e) => println!("Failed to read sizes: {}", e),
            }

            // Get detailed user list
            println!("\n--- User List ---");
            match zk.get_users() {
                Ok(users) => {
                    println!("Total users: {}", users.len());
                    for user in &users {
                        println!(
                            "  UID: {}, ID: {}, Name: {}, Privilege: {}, Card: {}",
                            user.uid, user.user_id, user.name, user.privilege, user.card
                        );
                    }
                }
                Err(e) => println!("Failed to get users: {}", e),
            }

            let _ = zk.disconnect();
        }
        Err(e) => println!("Failed to connect: {}", e),
    }
}
