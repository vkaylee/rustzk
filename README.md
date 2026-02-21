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
rustzk = "0.3.1"
```

## 🚀 Quick Start
... (keep existing quick start)

## 📖 Examples

The following examples are available in the `examples/` directory:

- `check_device_info.rs`: Basic device info, MAC, SN, and Capacity.
- `check_time.rs`: Check device time and drift.
- `check_timezone.rs`: **New!** Scan for timezone configuration (TZAdj, StandardTime).
- `get_attendance.rs`: Fetch historical attendance logs.
- `live_events.rs`: Monitor real-time clock-in events.
- `user_management.rs`: Add, delete, and list users.
- `fingerprint_management.rs`: Manage biometric templates.

Run any example using cargo:
```bash
cargo run --example check_timezone -- 192.168.1.201
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
