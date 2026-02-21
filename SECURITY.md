# Security Policy

## ⚠️ Protocol Warning

The ZK protocol used by these attendance devices is **inherently insecure**. It was designed for simplicity and internal networks, not for security in the modern sense.

### Known Weaknesses
- **Weak Authentication**: The `CommKey` authentication uses a simple XOR-based scramble (`make_commkey`) that is easily reversible.
- **No Encryption**: Data is transmitted in plain text (XORed but not encrypted with modern standards like TLS).
- **Session Hijacking**: Session IDs are only 16-bit and predictable.
- **UDP Vulnerabilities**: When using UDP, the protocol is susceptible to spoofing and replay attacks.

## Recommendations
1. **Isolated Networks**: Always deploy these devices on a separate, isolated VLAN with no direct access to the public internet or sensitive corporate networks.
2. **Firewalls**: Use strict firewall rules to allow communication only from authorized IP addresses.
3. **VPN**: If remote access is required, wrap the communication in a secure tunnel (e.g., WireGuard, OpenVPN).
4. **Physical Security**: Ensure physical access to the device is restricted, as many administrative functions can be triggered locally.

## Reporting a Vulnerability
If you discover a security vulnerability in **this library's implementation** (e.g., buffer overflows, memory safety issues), please open an issue on GitHub. 

Note that we cannot "fix" the inherent protocol weaknesses described above, as they are part of the device's design.
