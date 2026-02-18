# RustZK

A pure Rust implementation of the ZK attendance device protocol.

## 📦 Installation

Add `rustzk` to your `Cargo.toml`.

### From Crates.io

```toml
[dependencies]
rustzk = "0.1.0"
```

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
    
    // Set password if device requires authentication (default is 0)
    zk.set_password(12345);
    
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

## 🙏 Acknowledgements

This project is a Rust port of the [pyzk](https://github.com/fananimi/pyzk) library. Special thanks to the original authors for their work on the ZK protocol implementation.
