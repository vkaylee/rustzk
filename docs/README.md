# RustZK

A high-performance Rust port of the `pyzk2` library for interacting with ZKTeco biometric attendance devices.

## Features
- **Binary Protocol**: Native implementation of the ZK binary protocol (UDP/TCP).
- **Device Management**: Get firmware version, serial number, platform, and MAC address.
- **Data Retrieval**: Fetch users and attendance logs with automatic record size detection.
- **System Control**: Restart, power off, and door unlock capabilities.
- **Memory Analysis**: Real-time capacity and usage tracking for users, fingers, and records.

## Quick Start
```rust
use rustzk::ZK;

fn main() {
    let mut zk = ZK::new("192.168.1.201", 4370);
    if let Ok(_) = zk.connect(true) {
        let records = zk.get_attendance().unwrap();
        println!("Fetched {} records", records.len());
    }
}
```

## Documentation
See the [docs/](./docs/) directory for detailed information.
