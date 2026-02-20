use rustzk::models::User;
use rustzk::{ZKProtocol, USER_DEFAULT, ZK};
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

    println!("Connected!\n");

    // 1. Find a safe UID for testing (start from 65000 to avoid real users)
    println!("Detecting safe UID for testing (range 65000+)...");
    let next_uid = zk.get_next_free_uid(65000)?;
    println!("Selected safe UID: {}\n", next_uid);

    // 2. Create a new user object using the discovered UID
    let user_id_str = next_uid.to_string();
    let new_user = User {
        uid: next_uid,
        user_id: user_id_str.clone(), // Use same ID as UID for simplicity
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

    // 3. List users to verify
    println!("\nFetching user list...");
    let users = zk.get_users()?;
    if let Some(user) = users.iter().find(|u| u.uid == next_uid) {
        println!("Verified: User found in device list: {}", user.name);
    } else {
        println!("Warning: User not found in list immediately after adding.");
    }

    // 4. Update user by ID
    println!("\nDemonstrating Update by ID: {}", user_id_str);
    if let Some(mut user) = zk.find_user_by_id(&user_id_str)? {
        println!("Found User with UID: {}. Updating name...", user.uid);
        user.name = "Updated Rust Dev".into();
        zk.set_user(&user)?;
        println!("User updated successfully.");
    }

    // 5. Delete the user
    println!("\nDeleting user with UID {}...", next_uid);
    zk.delete_user(next_uid)?;
    println!("User deleted successfully.");

    zk.disconnect()?;
    Ok(())
}
