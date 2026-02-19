# Compliance: rustzk

## 1. GDPR (General Data Protection Regulation)
- **Data Processor:** `rustzk` acts as a processor of personal data (names, IDs, fingerprints, attendances).
- **Data Minimization:** `rustzk` only fetches data requested by the application.
- **Right to Erasure:** `rustzk` implements `delete_user` and `clear_attendance`, enabling full deletion of personal data from the device as required.
- **Data Protection by Design:** `rustzk` uses memory-safe Rust to prevent data leaks via buffer overruns.

## 2. CCPA (California Consumer Privacy Act)
- **Data Access:** `get_users` and `get_attendance` allow retrieving all personal information stored on the device for disclosure requests.
- **Deletion Rights:** `delete_user` supports the right to delete personal information.

## 3. Data Residency
- **Requirement:** `rustzk` does not transmit data to any third-party cloud. All data remains within the local network between the device and the application server.
- **Compliance:** Full control over data location.

## 4. Audit Trails
- **Requirement:** Logging of access to sensitive data.
- **Implementation:** `rustzk` itself does not log access, but the integrating application *must* log when `get_users` or `get_attendance` is called to maintain accountability.

## 5. Security Controls
- **NIST 800-53:**
    - **AC-3 (Access Enforcement):** Application-level access control to `rustzk` functions.
    - **SC-8 (Transmission Confidentiality):** Network-level encryption (VPN/VLAN) recommended as protocol lacks native encryption.
