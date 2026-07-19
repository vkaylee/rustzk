use rustzk::{ZKProtocol, ZK};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (ip, port) = rustzk::parse_ip_port();

    println!("=== ZK Fingerprint Management ===");
    println!("Connecting to {}:{}...", ip, port);

    let mut zk = ZK::new(&ip, port);
    zk.connect(ZKProtocol::Auto)?;

    println!(
        "Connected!
"
    );

    // 1. Fetch all templates
    println!("Fetching all fingerprint templates...");
    let templates = zk.get_templates()?;
    println!(
        "Found {} templates on the device.
",
        templates.len()
    );

    if !templates.is_empty() {
        println!(
            "{:<6} {:<6} {:<10} {:<15}",
            "UID", "FID", "Valid", "Size (Bytes)"
        );
        println!("{}", "-".repeat(40));

        // Show first 20 templates
        for temp in templates.iter().take(20) {
            println!(
                "{:<6} {:<6} {:<10} {:<15}",
                temp.uid(),
                temp.fid(),
                temp.valid(),
                temp.template().len()
            );
        }

        if templates.len() > 20 {
            println!("... (and {} more)", templates.len() - 20);
        }
    } else {
        println!("No fingerprints found.");
    }

    zk.disconnect()?;
    Ok(())
}
