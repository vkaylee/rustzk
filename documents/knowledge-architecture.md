# System Architecture: rustzk

## Overview
`rustzk` is designed as a layered library, following clean architecture principles. It abstracts the complexity of the proprietary ZK protocol into a simple, high-level API for developers.

## Core Components

### 1. `ZK` Struct (Client)
- **Role:** Main entry point for interacting with the device.
- **Responsibilities:**
    - Connect/Disconnect.
    - Send commands (CMD_...) and receive responses (ACK_...).
     - Manage session state (session_id, reply_id).
    - Handle protocol negotiation (TCP vs UDP).

### 2. Protocol Layer (`protocol` module)
- **Role:** Handles the specifics of the ZK communication protocol.
- **Components:**
    - `ZKPacket`: Represents a single command or data packet.
     - `TCPWrapper`: Handles TCP-specific framing (header length, checksum).
    - `UDPWrapper`: Handles UDP-specific datagrams.

### 3. Transport Abstraction (`transport` module - implicit)
- **Role:** Abstracts the underlying network socket.
- **Traits:** `Read`, `Write` (std::io).
- **Implementation:** `TcpStream`, `UdpSocket`.

### 4. Data Models (`models` module)
- **Role:** Represents domain entities.
- **Entities:**
    - `User`: User information (ID, name, privilege, etc.).
    - `Finger`: Fingerprint templates.
    - `Attendance`: Attendance log records.
    - `Time`: Device time structure.

### 5. Error Handling (`error` module)
- **Role:** Centralized error management using `thiserror`.
- **Types:** `ZKError` (Network, Protocol, Timeout, Data, etc.).

## Data Flow
1. **Request:** Application calls `zk.get_users()`.
2. **Command Construction:** `ZK` builds a `ZKPacket` with `CMD_USER_RRQ`.
3. **Transport:** `TCPWrapper` adds framing and sends via `TcpStream`.
4. **Device Response:** Device returns strict binary data.
5. **Parsing:** `ZK` receives raw bytes, validates checksum/header.
6. **Deserialization:** Raw bytes are parsed into `User` structs based on fixed-length or variable-length formats.
7. **Result:** `Vec<User>` is returned to the application.

## Design Patterns
- **Builder Pattern:** Used implicitly in constructing `ZK` clients.
- **Adapter Pattern:** Adapting TCP/UDP streams to a unified interface.
- **Error Handling:** Result types with custom `ZKError` enum.
