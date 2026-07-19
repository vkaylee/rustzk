use rustzk::{ZKProtocol, ZK};

fn main() {
    let (ip, port) = rustzk::parse_ip_port();

    let mut zk = ZK::new(&ip, port);
    if zk.connect(ZKProtocol::Auto).is_ok() {
        let potential_keys = vec!["TZAdj", "~Tz", "StandardTime", "TimeZone", "DayLightTime"];

        println!("Scanning for timezone related options...");
        for key in potential_keys {
            match zk.get_option_value(key) {
                Ok(val) => println!("  Option '{}': {}", key, val),
                Err(_) => println!("  Option '{}': Not supported", key),
            }
        }

        match zk.get_timezone() {
            Ok(tz) => println!("\nDetected Timezone Offset: {} hours", tz),
            Err(_) => println!("\nCould not detect timezone via get_timezone()"),
        }
    }
}
