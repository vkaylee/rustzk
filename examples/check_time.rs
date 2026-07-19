use chrono::Local;
use rustzk::{ZKProtocol, ZK};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (ip, port) = rustzk::parse_ip_port();

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
