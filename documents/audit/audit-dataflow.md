# Data Flow Diagram: rustzk

## 1. High-Level Data Flow
1. **Application Layer** -> `rustzk::ZK` (Request: `get_attendance`)
2. **Protocol Layer** -> `protocol::TCPWrapper` (Encapsulate `CMD_ATTLOG_RRQ`)
3. **Transport Layer** -> `TcpStream` (Send Packet)
4. **Network Layer** -> Switch/Router -> **Device**
5. **Device** -> Access DB (Internal Flash) -> Serialize Data -> Send Packet
6. **Network Layer** -> Switch/Router -> **rustzk**
7. **Transport Layer** -> `TcpStream` (Receive Bytes)
8. **Protocol Layer** -> `protocol::ZKPacket` (Parse Header/Checksum)
9. **Data Layer** -> `models::Attendance` (Deserialize Payload)
10. **Application Layer** -> `Vec<Attendance>` (Result)

## 2. Sensitive Data Handling
- **User IDs (PII):** Transmitted unencrypted.
- **Names (PII):** Transmitted unencrypted.
- **Fingerprints (Biometric):** Transmitted as raw binary templates. Obfuscation is minimal.
- **Passwords (Credential):** Transmitted as hashes, but potentially crackable if weak algorithms are used by the device firmware.

## 3. Storage
- `rustzk` is stateless regarding user data.
- It acts purely as a conduit between the application and the device.
- **No caching** of sensitive data occurs within the library itself.

## 4. Encryption Boundaries
- **Encrypted:** None (by default).
- **Trusted Zone:** The local network segment containing the device and the application server.
- **Untrusted Zone:** Any network beyond the local gateway.
