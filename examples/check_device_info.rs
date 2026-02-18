use rustzk::{ZKProtocol, ZK};
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let (ip, port) = if args.len() >= 3 {
        (args[1].clone(), args[2].parse().unwrap_or(4370))
    } else {
        ("192.168.12.14".to_string(), 4370)
    };

    println!("Checking device info for {}:{}...", ip, port);

    let mut zk = ZK::new(&ip, port);

    if let Err(e) = zk.connect(ZKProtocol::Auto) {
        eprintln!("Failed to connect: {}", e);
        std::process::exit(1);
    }

    println!("Connected!\n");

    match zk.get_mac() {
        Ok(mac) => println!("MAC Address: {}", mac),
        Err(e) => println!("Failed to get MAC: {}", e),
    }

    match zk.get_firmware_version() {
        Ok(fw) => println!("Firmware Version: {}", fw),
        Err(e) => println!("Failed to get Firmware: {}", e),
    }

    match zk.get_time() {
        Ok(t) => println!("Device Time: {}", t),
        Err(e) => println!("Failed to get Time: {}", e),
    }

    match zk.get_platform() {
        Ok(p) => println!("Platform: {}", p),
        Err(e) => println!("Failed to get Platform: {}", e),
    }

    match zk.get_face_version() {
        Ok(v) => println!("Face Algo Version: {}", v),
        Err(e) => println!("Failed to get Face Algo: {}", e),
    }

    match zk.get_fp_version() {
        Ok(v) => println!("Fingerprint Algo Version: {}", v),
        Err(e) => println!("Failed to get FP Algo: {}", e),
    }
}
