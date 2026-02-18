use rustzk::ZK;

fn main() {
    let mut zk = ZK::new("192.168.1.201", 4370);

    println!("Connecting to ZK machine...");
    match zk.connect(true) {
        Ok(_) => {
            println!("Connected successfully!");

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

            if zk.read_sizes().is_ok() {
                println!("\n--- Device Capacity ---");
                println!("Users: {} / {}", zk.users, zk.users_cap);
                println!("Fingers: {} / {}", zk.fingers, zk.fingers_cap);
                println!("Attendance Records: {} / {}", zk.records, zk.rec_cap);
                if zk.faces_cap > 0 {
                    println!("Faces: {} / {}", zk.faces, zk.faces_cap);
                }
            }

            println!("\nFetching attendance records...");
            match zk.get_attendance() {
                Ok(records) => {
                    println!("Found {} records", records.len());
                    for record in records.iter().take(10) {
                        println!("{:?}", record);
                    }
                }
                Err(e) => println!("Failed to get attendance: {}", e),
            }

            let _ = zk.disconnect();
        }
        Err(e) => println!("Failed to connect: {}", e),
    }
}
