# Domain Model: rustzk

## Entities

### User (`User` struct)
Represents a user account on the device.
- **`uid` (u32):** Internal user ID.
- **`user_id` (String):** User ID displayed on the device (often employee ID).
- **`name` (String):** User's full name.
- **`privilege` (u8):** User privilege level (USER_DEFAULT, USER_ENROLLER, USER_MANAGER, USER_ADMIN).
- **`password` (String):** User password hash (usually empty if not set).
- **`group_id` (u8):** User group ID.
- **`card` (u32):** RFID card number.
- **`timezone_type` (u8):** Timezone type (usually 0).

### Fingerprint (`Finger` struct)
Represents a fingerprint template.
- **`uid` (u32):** User ID associated with the fingerprint.
- **`fid` (u8):** Finger ID (0-9).
- **`valid` (u8):** Validity flag.
- **`template` (Vec<u8>):** Raw fingerprint template data.

### Attendance (`Attendance` struct)
Represents a single attendance log entry.
- **`user_id` (String):** User ID associated with the log.
- **`status` (u8):** Attendance status (Check-in, Check-out, Break-out, Break-in, OT-in, OT-out).
- **`timestamp` (DateTime<FixedOffset>):** Date and time of the log, with timezone offset applied.
- **`punch` (u8):** Punch type (0 usually).
- **`workcode` (u32):** Work code associated with the log.
- **`timezone_offset` (i32):** Device timezone offset in minutes.

### Device Info
- **`firmware_version` (String):** Device firmware version.
- **`serial_number` (String):** Device serial number.
- **`mac_address` (String):** Device MAC address.
- **`face_version` (u8):** Face algorithm version.
- **`fp_version` (u8):** Fingerprint algorithm version.
- **`platform` (String):** Device platform name.
- **`time` (DateTime<FixedOffset>):** Current device time.

## Relationships
- **User -> Finger:** One-to-Many (One user can have multiple fingerprints, up to 10).
- **User -> Attendance:** One-to-Many (One user can have multiple attendance logs).
- **Attendance -> Time:** Each attendance log has a specific timestamp.

## Constants
- **Privilege Levels:**
    - `USER_DEFAULT`: Standard user.
    - `USER_ENROLLER`: Can enroll users.
    - `USER_MANAGER`: Can manage users.
    - `USER_ADMIN`: Full administrative access.
- **Attendance Status:**
    - `0`: Check-in
    - `1`: Check-out
    - `2`: Break-out
    - `3`: Break-in
    - `4`: OT-in
    - `5`: OT-out
