use rustzk::{ZKProtocol, ZK};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (ip, port) = rustzk::parse_ip_port();

    println!("=== ZK Real-time Event Monitor ===");
    println!("Connecting to {}:{}...", ip, port);

    let mut zk = ZK::new(&ip, port);
    // Set a shorter timeout for responsive idle detection
    zk.set_timeout(std::time::Duration::from_secs(5));
    zk.connect(ZKProtocol::Auto)?;

    println!(
        "Connected! Monitoring events... (Press Ctrl+C to stop)
"
    );

    // Start listening for events
    let event_iter = zk.listen_events()?;

    for event in event_iter {
        match event {
            Ok(log) => {
                println!(
                    "NEW EVENT: User: {:<10} | Time: {:<25} | Status: {} | Punch: {}",
                    log.user_id(),
                    log.iso_format(),
                    log.status(),
                    log.punch()
                );
            }
            Err(e) => {
                // Ignore timeouts during idle, but report other errors
                eprintln!("Error receiving event: {}", e);
            }
        }
    }

    Ok(())
}
