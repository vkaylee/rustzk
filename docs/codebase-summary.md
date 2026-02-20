# Codebase Summary

## Core Modules
- **`src/lib.rs`**: Main entry point, `ZK` client implementation. Now includes robust memory safety checks, improved authentication error handling, and device time synchronization logic (`get_time`, `set_time`).
- **`src/constants.rs`**: ZK Protocol command codes, flags, and safety limits (e.g., `MAX_RESPONSE_SIZE`).
- **`src/models.rs`**: Domain models (User, Attendance, Finger).
- **`src/protocol.rs`**: Packet serialization, checksums, and transport wrapping. Uses a zero-copy design with `Cow` for memory efficiency.

## Data Structures
- `ZK`: Orchestrates connection state and device communication.
- `ZKPacket`: Internal representation of a protocol packet. Uses lifetimes and `Cow` to avoid unnecessary payload cloning.
- `User`: Represents a device user (UID, Name, Privilege, etc.).
- `Attendance`: Represents a clock-in/out record.

## Testing Suite
- **`tests/client_tests.rs`**: Integration tests using a stateful Mock Server to verify full connection and data retrieval flows.
- **`tests/parsing_tests.rs`**: Unit tests for decoding raw byte streams into model structures.
- **`examples/check_time.rs`**: Practical demonstration of localized time retrieval and drift analysis.

## CI/CD Workflows
- **`.github/workflows/ci.yml`**: Runs CI checks on every push/PR to master.
- **`.github/workflows/publish.yml`**: Automatically publishes to crates.io when a version tag (`v*`) is pushed.
