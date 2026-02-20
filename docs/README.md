# RustZK

A high-performance Rust port of the `pyzk2` library for interacting with ZKTeco biometric attendance devices.

## Features
- **High Performance**: Zero-copy packet parsing and buffer-centric data retrieval for minimal allocation overhead.
- **Binary Protocol**: Pure Rust implementation of the ZK binary protocol (UDP/TCP).
- **Automated Localization**: Lazy on-demand timezone synchronization (`TZAdj`) for accurate, localized timestamps.
- **Device Management**: Get firmware version, serial number, platform, and MAC address.
- **Data Retrieval**: Fetch users and attendance logs with automatic record size detection.
- **System Control**: Restart, power off, and door unlock capabilities.
- **Memory Analysis**: Real-time capacity and usage tracking for users, fingers, and records.

## Quick Start
```rust
use rustzk::{ZK, ZKProtocol};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut zk = ZK::new("192.168.1.201", 4370);
    
    // Connect with auto-protocol detection
    zk.connect(ZKProtocol::Auto)?;
    
    // Automated localization: timezone is synced on first time-related call
    let time = zk.get_time()?;
    println!("Device Time: {}", time.to_rfc3339());

    let records = zk.get_attendance()?;
    println!("Fetched {} records", records.len());
    
    zk.disconnect()?;
    Ok(())
}
```

## Documentation
- [Project Overview](project-overview.md)
- [API Reference](api-reference.md)
- [System Architecture](system-architecture.md)
- [Code Standards](code-standards.md)
- [Codebase Summary](codebase-summary.md)
