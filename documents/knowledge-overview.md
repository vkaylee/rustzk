# Project Overview: rustzk

## Purpose
`rustzk` is a pure Rust implementation of the ZK network protocol (UDP/TCP) used for communicating with ZKTeco time and attendance devices. It provides a robust, type-safe, and asynchronous-ready client library for developers to integrate biometric devices into their applications without relying on the official SDK.

## Key Features
- **Protocol Support:** Full implementation of both TCP and UDP protocols for ZK devices.
- **Biometric Data:** Retrieve user data, fingerprints, and face templates.
- **Attendance Management:** Fetch attendance logs with automatic timezone support (offset + ISO-8601).
- **Device Control:** Command execution (reboot, power off, clear data, voice test, etc.).
- **Robustness:** Handles fragmented packets, connection retries, and protocol quirks.
- **Cross-Platform:** Pure Rust implementation, runs on Linux, macOS, and Windows.

## Technology Stack
- **Language:** Rust (Edition 2021)
- **Core Dependencies:**
    - `byteorder`: Endianness handling for binary protocol.
    - `chrono`: Date and time manipulation.
    - `thiserror`: Error handling.
    - `encoding_rs` (potential): For handling device-specific string encodings (e.g., GBK).

## Getting Started

### Prerequisites
- Rust toolchain (stable) installed.
- A ZKTeco device reachable via network (default port 4370).

### Installation
Add `rustzk` to your `Cargo.toml`:
```toml
[dependencies]
rustzk = { path = "/path/to/rustzk" } # Or git repository
```

### Basic Usage
```rust
use rustzk::{ZK, ZKProtocol};

fn main() {
    let mut zk = ZK::new("192.168.1.201", 4370);
    if let Ok(_) = zk.connect(ZKProtocol::TCP) {
        println!("Connected!");
        let logs = zk.get_attendance().unwrap();
        for log in logs {
            println!("{}", log);
        }
    }
}
```
