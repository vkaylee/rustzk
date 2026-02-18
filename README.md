# RustZK

A pure Rust implementation of the ZK attendance device protocol.

## 📦 Installation

Add `rustzk` to your `Cargo.toml`.

### From GitHub

```toml
[dependencies]
rustzk = { git = "https://github.com/vkaylee/rustzk" }
```

### Local Path (for development within workspace)

```toml
[dependencies]
rustzk = { path = "../rustzk" }
```

## 🚀 Usage Example

```rust
use rustzk::ZK;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut zk = ZK::new("192.168.1.201", 4370);
    
    // Connect to device
    zk.connect(true)?; // true for TCP, false for UDP
    
    // Get user list
    let users = zk.get_users()?;
    println!("Found {} users", users.len());
    
    // Get attendance logs
    let logs = zk.get_attendance()?;
    println!("Found {} attendance records", logs.len());

    // Disconnect
    zk.disconnect()?;
    
    Ok(())
}
```

## 📜 License

MIT
