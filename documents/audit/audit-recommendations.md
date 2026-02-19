# Security Recommendations: rustzk

## 1. Network Isolation (Critical)
- **Implement:** VLAN Segmentation.
- **Why:** ZK devices use insecure protocols. Isolating them prevents attackers from accessing corporate networks or sniffing traffic.
- **Action:** Configure switch ports for devices to be on a separate VLAN with no internet access.

## 2. Firewall Rules
- **Implement:** Host-based firewall on the application server.
- **Why:** To restrict device communication to only authorized IPs.
- **Action:** Allow inbound/outbound on port 4370/UDP and 4370/TCP only for the device IP range.

## 3. Strong Authentication
- **Implement:** Change default Comm Key.
- **Why:** Prevents unauthorized control.
- **Action:** Set a non-zero, 6-digit Comm Key on each device via the device menu.

## 4. Physical Security
- **Implement:** Secure device mounting.
- **Why:** Prevents physical tampering or theft.
- **Action:** Ensure devices are mounted securely and have tamper alarms enabled.

## 5. Regular Audits
- **Implement:** Periodic user review.
- **Why:** Detect unauthorized accounts.
- **Action:** Compare device user list with HR system monthly.

## 6. Firmware Updates
- **Implement:** Vendor patch management.
- **Why:** Fix known vulnerabilities.
- **Action:** Check ZKTeco support for firmware updates compatible with your device model.
