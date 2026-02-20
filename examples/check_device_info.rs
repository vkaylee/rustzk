use rustzk::{ZKProtocol, ZK};
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <ip> [port]", args[0]);
        eprintln!("Example: {} 192.168.1.201 4370", args[0]);
        std::process::exit(1);
    }
    let ip = args[1].clone();
    let port = if args.len() > 2 {
        args[2].parse().unwrap_or(4370)
    } else {
        4370
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
        Ok(t) => println!("Device Time: {} (ISO: {})", t, t.to_rfc3339()),
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
