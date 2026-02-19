# Feature Specifications (Specs): rustzk

## 1. Connection Management
- **Feature:** Open/Close TCP and UDP connections.
- **Spec:**
    - `ZK::new(ip, port)` initializes the client.
    - `zk.connect(protocol)` attempts to establish a socket.
    - `zk.disconnect()` closes the socket cleanly.
    - **Retry Policy:** Automatic retries for intermittent failures (configurable).

## 2. User Administration
- **Feature:** Manage user accounts.
- **Spec:**
    - `get_users()`: Returns a list of all users.
    - `set_user(user)`: Creates or updates a user.
    - `delete_user(uid)`: Removes a user by UID.
    - **Data:** Should handle name encoding (ASCII/UTF-8/GBK).

## 3. Attendance Retrieval
- **Feature:** Fetch attendance logs.
- **Spec:**
    - `get_attendance()`: Retrieves all stored logs (ATTLOG).
    - **Format:** Returns `Attendance` structs with ISO-8601 timestamps.
    - **Handling:** Manages large datasets by reading in chunks (default 64KB).
    - **Optimization:** Automatic buffer resizing for efficiency.
    - **Timezone:** Applies device timezone offset to raw timestamps.

## 4. Device Information
- **Feature:** Query device properties.
- **Spec:**
    - `get_time()`: Current device time.
    - `get_serial_number()`: Unique device identifier.
    - `get_mac()`: Network interface MAC address.
    - `get_firmware_version()`: Installed firmware version.
    - `get_platform()`: Hardware platform (e.g., ZM100).
    - `get_face_version()`: Face algorithm version.
    - `get_fp_version()`: Fingerprint algorithm version.

## 5. Device Control
- **Feature:** Remote administration commands.
- **Spec:**
    - `restart()`: Reboots the device.
    - `poweroff()`: Shuts down the device.
    - `enable_device()`: Unlocks the UI/keypad.
    - `disable_device()`: Locks the UI/keypad.
    - `clear_attendance()`: Deletes all attendance logs.
    - `test_voice()`: Plays a test sound.
