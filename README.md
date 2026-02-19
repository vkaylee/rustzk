# RustZK

A robust, pure Rust implementation of the ZK attendance device protocol (TCP/UDP).

[![Crates.io](https://img.shields.io/crates/v/rustzk.svg)](https://crates.io/crates/rustzk)
[![License](https://img.shields.io/crates/l/rustzk.svg)](LICENSE)

## 🌟 Key Features

- **Protocol Support:** Full TCP and UDP implementation.
- **Biometric Data:** Retrieve users, fingerprints, and face templates.
- **Timezone Aware:** Automatic device timezone synchronization and ISO-8601 timestamps (e.g., `2023-10-27T10:00:00+07:00`).
- **Robustness:** Handles fragmented packets, connection retries, and protocol quirks.
- **Cross-Platform:** Runs on Linux, macOS, and Windows with zero C dependencies.

## 📦 Installation

Add `rustzk` to your `Cargo.toml`:

```toml
[dependencies]
rustzk = "0.2.4"
```

## 🚀 Quick Start

```rust
use rustzk::{ZK, ZKProtocol};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Initialize client
    let mut zk = ZK::new("192.168.1.201", 4370);
    
    // 2. Connect (Auto-detect protocol)
    zk.connect(ZKProtocol::Auto)?;
    
    // 3. Get device time (ISO-8601 with timezone)
    let time = zk.get_time()?;
    println!("Device Time: {}", time.to_rfc3339());
    
    // 4. Get attendance logs
    let logs = zk.get_attendance()?;
    for log in logs {
        println!("User: {}, Time: {}, Status: {}", 
            log.user_id, 
            log.iso_format(), 
            log.status
        );
    }

    // 5. Disconnect
    zk.disconnect()?;
    Ok(())
}
```

## 📚 Documentation

Detailed documentation is available in the `documents/` directory:

### Core Concepts
- [Overview](documents/knowledge-overview.md) - Goals and architecture.
- [Domain Model](documents/knowledge-domain.md) - Users, attendance, and fingerprints.
- [Source Code](documents/knowledge-source-base.md) - Project structure.

### Business & Features
- [Product Requirements](documents/business/business-prd.md)
- [Feature Specs](documents/business/business-features.md)
- [Workflows](documents/business/business-workflows.md)
- [Glossary](documents/business/business-glossary.md)

### Security & Audit
- [Security Assessment](documents/audit/audit-security.md)
- [Compliance](documents/audit/audit-compliance.md)
- [Data Flow](documents/audit/audit-dataflow.md)
- [Recommendations](documents/audit/audit-recommendations.md)

## 🤝 Contributing

Contributions are welcome! Please check [Standards](documents/knowledge-standards.md) for coding guidelines.

## 📜 License

MIT License. See [LICENSE](LICENSE) for details.

## 🙏 Acknowledgements

This project is inspired by the [pyzk](https://github.com/fananimi/pyzk) library.
