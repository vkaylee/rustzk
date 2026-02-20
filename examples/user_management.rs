use rustzk::models::User;
use rustzk::{ZKProtocol, ZK, USER_DEFAULT};
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

    println!("=== ZK User Management ===");
    println!("Connecting to {}:{}...", ip, port);

    let mut zk = ZK::new(&ip, port);
    zk.connect(ZKProtocol::Auto)?;

    println!(
        "Connected!
"
    );

    // Detect user packet size by fetching users (required for correct set_user format)
    println!("Detecting device configuration...");
    let _ = zk.get_users();
    println!("User packet size: {} bytes
", zk.user_packet_size);

    // 1. Create a new user object
    let new_user = User {
        uid: 65500,                // High UID to avoid collision
        user_id: "65500".into(),   // Same as UID
        name: "Rust Dev".into(),
        privilege: USER_DEFAULT,
        password: "".into(),
        group_id: "1".into(),
        card: 0,
    };

    println!(
        "Adding user: {} (ID: {}, UID: {})",
        new_user.name, new_user.user_id, new_user.uid
    );
    zk.set_user(&new_user)?;
    println!("User added successfully.");

    // 2. List users to verify
    println!(
        "
Fetching user list..."
    );
    let users = zk.get_users()?;
    if let Some(user) = users.iter().find(|u| u.uid == 65500) {
        println!("Verified: User found in device list: {}", user.name);
    } else {
        println!("Warning: User not found in list immediately after adding.");
    }

    // 3. Delete the user
    println!(
        "
Deleting user with UID 65500..."
    );
    zk.delete_user(65500)?;
    println!("User deleted successfully.");

    zk.disconnect()?;
    Ok(())
}
