use rustzk::{ZKProtocol, ZK};
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let ip = args.get(1).expect("IP required");

    let mut zk = ZK::new(ip, 4370);
    if let Ok(_) = zk.connect(ZKProtocol::Auto) {
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
