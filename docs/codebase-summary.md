# Codebase Summary

## Core Modules
- **`src/lib.rs`**: Main entry point, `ZK` client implementation. Now includes robust memory safety checks, improved authentication error handling, and device time synchronization logic (`get_time`, `set_time`).
- **`src/constants.rs`**: ZK Protocol command codes, flags, safety limits, and centralized packet/record offsets.
- **`src/models.rs`**: Domain models (User, Attendance, Finger).
- **`src/protocol.rs`**: Packet serialization, transport wrapping, and aligned checksum logic matching the reference protocol implementation. Uses a zero-copy design with `Cow` for memory efficiency.

## Data Structures
- `ZK`: Orchestrates connection state and device communication.
- `ZKPacket`: Internal representation of a protocol packet. Uses lifetimes and `Cow` to avoid unnecessary payload cloning.
- `User`: Represents a device user (UID, Name, Privilege, etc.).
- `Attendance`: Represents a clock-in/out record.

## Testing Suite
- **`tests/client_tests.rs`**: Integration tests using a stateful Mock Server to verify full connection and data retrieval flows.
- **`tests/parsing_tests.rs`**: Unit tests for decoding raw byte streams into model structures.
- **`tests/user_tests.rs`**: Comprehensive suite for user administration (add, delete, conflict detection).
- **`tests/fingerprint_tests.rs`**: Verification of biometric data retrieval and deletion logic.
- **`tests/drop_tests.rs`**: Verification of resource cleanup and non-blocking `Drop` behavior.
- **`tests/live_event_tests.rs`**: Integration tests for real-time monitoring across various data formats.
- **`tests/time_sync_tests.rs`**: Specialized tests for verifying lazy timezone synchronization and offset caching logic.
- **`examples/check_time.rs`**: Practical demonstration of localized time retrieval and drift analysis.
- **`examples/user_management.rs`**: Demonstration of the complete user administration lifecycle.
- **`examples/fingerprint_management.rs`**: Practical guide to handling biometric templates.
- **`examples/live_events.rs`**: Real-time event monitoring implementation guide.

## CI/CD Workflows
- **`.github/workflows/ci.yml`**: Runs CI checks on every push/PR to master.
- **`.github/workflows/publish.yml`**: Automatically publishes to crates.io when a version tag (`v*`) is pushed.
