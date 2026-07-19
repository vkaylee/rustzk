use rustzk::{ZKProtocol, ZK};

fn main() {
    env_logger::init();
    let (ip, port) = rustzk::parse_ip_port();

    println!("Checking device info for {}:{}...", ip, port);

    let mut zk = ZK::new(&ip, port);

    if let Err(e) = zk.connect(ZKProtocol::Auto) {
        eprintln!("Failed to connect: {}", e);
        std::process::exit(1);
    }

    println!("Connected!\n");

    if let Err(e) = zk.read_sizes() {
        eprintln!("Failed to read device sizes: {}", e);
    } else {
        println!("--- Device Capacity & Usage ---");
        println!("Users:      {} / {}", zk.users(), zk.users_cap());
        println!("Fingers:    {} / {}", zk.fingers(), zk.fingers_cap());
        println!("Records:    {} / {}", zk.records(), zk.records_cap());
        println!("Faces:      {} / {}", zk.faces(), zk.faces_cap());
        println!("Cards:      {}", zk.cards());
        println!("-------------------------------\n");
    }

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
