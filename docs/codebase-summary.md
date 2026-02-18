# Codebase Summary

## Core Modules
- **`src/lib.rs`**: Main entry point, `ZK` client implementation. Now includes robust memory safety checks and improved authentication error handling.
- **`src/constants.rs`**: ZK Protocol command codes, flags, and safety limits (e.g., `MAX_RESPONSE_SIZE`).
- **`src/models.rs`**: Domain models (User, Attendance, Finger).
- **`src/protocol.rs`**: Packet serialization, checksums, and transport wrapping.

## Data Structures
- `ZK`: Orchestrates connection state and device communication.
- `ZKPacket`: Internal representation of a protocol packet.
- `User`: Represents a device user (UID, Name, Privilege, etc.).
- `Attendance`: Represents a clock-in/out record.

## Testing Suite
- **`tests/client_tests.rs`**: Integration tests using a stateful Mock Server to verify full connection and data retrieval flows.
- **`tests/parsing_tests.rs`**: Unit tests for decoding raw byte streams into model structures.
