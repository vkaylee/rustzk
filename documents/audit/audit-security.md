# Security Audit: rustzk

## 1. Executive Summary
`rustzk` is a client-side library interacting with proprietary hardware devices. The primary security model relies on the assumption that the local network is secure, as the ZK protocol itself lacks strong encryption or authentication mechanisms.

## 2. Vulnerability Analysis

### Network Communication
- **Protocol:** The ZK protocol transmits data (including user IDs and attendance logs) in cleartext or with weak obfuscation.
- **Risk:** Man-in-the-Middle (MitM) attacks can easily intercept sensitive employee data.
- **Mitigation:**
    - Isolate devices on a dedicated VLAN.
    - Restrict access to the device port (4370) via firewall rules.
    - **Do not** expose devices directly to the public internet.

### Authentication
- **Mechanism:** Devices use a numeric password (comm key) for connection, defaulting to 0.
- **Risk:** Weak or default session keys allow unauthorized command execution (e.g., deleting users).
- **Mitigation:**
    - Change the default Comm Key on all devices.
    - `rustzk` supports setting a custom password locally, but the protocol limit (0-999999) is susceptible to brute-force.

### Input Validation
- **Risk:** Malformed packets from a malicious device could exploit buffer overflows in the parsing logic.
- **Mitigation:** `rustzk` uses safe Rust, which prevents memory corruption vulnerabilities. Bounds checking is enforced during packet decoding.

## 3. Findings
| ID | Severity | Description | Status |
|----|----------|-------------|--------|
| S1 | High | Cleartext communication protocol. | Inherent to device |
| S2 | Medium | Weak authentication (Comm Key). | User Configurable |
| S3 | Low | Lack of server-side identity verification. | Inherent to device |

## 4. Conclusion
While `rustzk` implements memory-safe parsing, the underlying security depends heavily on network isolation. It is strongly recommended to treat ZK devices as untrusted endpoints.
