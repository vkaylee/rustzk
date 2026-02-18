# Codebase Summary

## Core Modules
- **`src/lib.rs`**: Main entry point, `ZK` client implementation.
- **`src/constants.rs`**: ZK Protocol command codes and flags.
- **`src/models.rs`**: Domain models (User, Attendance, Finger).
- **`src/protocol.rs`**: Packet serialization, checksums, and transport wrapping.

## Data Structures
- `ZK`: Orchestrates connection state and device communication.
- `ZKPacket`: Internal representation of a protocol packet.
- `User`: Represents a device user (UID, Name, Privilege, etc.).
- `Attendance`: Represents a clock-in/out record.
