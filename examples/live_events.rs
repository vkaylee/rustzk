use rustzk::{ZKProtocol, ZK};
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
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

    println!("=== ZK Real-time Event Monitor ===");
    println!("Connecting to {}:{}...", ip, port);

    let mut zk = ZK::new(&ip, port);
    // Set a shorter timeout for responsive idle detection
    zk.timeout = std::time::Duration::from_secs(5);
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
                    log.user_id,
                    log.iso_format(),
                    log.status,
                    log.punch
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
