use chrono::Local;
use rustzk::{ZKProtocol, ZK};
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let (ip, port) = if args.len() >= 3 {
        (args[1].clone(), args[2].parse().unwrap_or(4370))
    } else if args.len() == 2 {
        (args[1].clone(), 4370)
    } else {
        eprintln!("Usage: {} <ip> [port]", args[0]);
        eprintln!("Example: {} 192.168.12.13 4370", args[0]);
        std::process::exit(1);
    };

    println!("=== ZK Time Checker ===");
    println!("Connecting to {}:{}...", ip, port);

    let mut zk = ZK::new(&ip, port);

    // Automated localization: timezone is synced during connect()
    zk.connect(ZKProtocol::Auto)?;

    println!(
        "Connected!
"
    );

    // Fetch device time
    let device_time = zk.get_time()?;
    let local_now = Local::now();

    println!("Device Time (Localized) : {}", device_time.to_rfc3339());
    println!("Local Machine Time      : {}", local_now.to_rfc3339());

    let drift = (device_time.timestamp() - local_now.timestamp()).abs();
    println!("Time Drift              : {} seconds", drift);

    if drift > 60 {
        println!(
            "
[!] Significant drift detected (> 1 min)."
        );
    } else {
        println!(
            "
[✓] Device time is reasonably synced."
        );
    }

    zk.disconnect()?;
    Ok(())
}
