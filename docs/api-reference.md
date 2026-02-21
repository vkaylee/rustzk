# API Reference

## ZK Struct

### `pub fn new(addr: &str, port: u16) -> Self`
Creates a new ZK instance.

### `pub fn set_password(&mut self, password: u32)`
Sets the communication password (CommKey) for the device. If the device requires authentication, this password will be used automatically during the `connect()` handshake. Default is 0.

### `pub fn change_password(&mut self, new_password: u32) -> ZKResult<()>`
Updates the internal communication password (CommKey) of the device to a new value. **Note:** After a successful change, you must use the new password for all future connection attempts.

### `pub fn connect(&mut self, protocol: ZKProtocol) -> ZKResult<()>`
Connects to the device using the specified protocol.

**`ZKProtocol` Variants:**
- `TCP`: Force TCP connection.
- `UDP`: Force UDP connection.
- `Auto`: Attempt TCP first, fallback to UDP.

### `pub fn get_attendance(&mut self) -> ZKResult<Vec<Attendance>>`
Retrieves all historical attendance logs from the device.

### `pub fn listen_events(&mut self) -> ZKResult<impl Iterator<Item = ZKResult<Attendance>> + '_>`
Registers for real-time attendance events and returns an iterator that yields logs as they happen. This is a blocking operation when iterating.

### `pub fn get_templates(&mut self) -> ZKResult<Vec<Finger>>`
Retrieves all fingerprint templates currently stored on the device.

### `pub fn get_user_template(&mut self, uid: u16, fid: u8) -> ZKResult<Option<Finger>>`
Retrieves a specific fingerprint template for a given user (UID) and finger index (FID).

### `pub fn delete_user_template(&mut self, uid: u16, fid: u8) -> ZKResult<()>`
Deletes a specific fingerprint template from the device.

### `pub fn set_option(&mut self, key: &str, value: &str) -> ZKResult<()>`
Sets a system option on the device (e.g., `DeviceName`).

### `pub fn get_option_value(&mut self, key: &str) -> ZKResult<String>`
Retrieves a raw configuration option from the device by its key name (e.g., `~SerialNumber`, `TZAdj`).

### `pub fn get_timezone(&mut self) -> ZKResult<i32>`
Retrieves the device timezone adjustment (usually in hours, e.g., `7` for UTC+7). This actively queries the device's `TZAdj` configuration parameter.

### `pub fn get_serial_number(&mut self) -> ZKResult<String>`
Retrieves the device serial number.

### `pub fn get_mac(&mut self) -> ZKResult<String>`
Retrieves the device MAC address.

### `pub fn get_firmware_version(&mut self) -> ZKResult<String>`
Retrieves the device firmware version.

### `pub fn get_platform(&mut self) -> ZKResult<String>`
Retrieves the device platform name (e.g., `ZAM180_TFT`).

### `pub fn get_time(&mut self) -> ZKResult<DateTime<FixedOffset>>`
Retrieves the current device time. **Note:** On the first call, this method automatically fetches and caches the device's timezone offset (`TZAdj`) to provide localized results.

### `pub fn set_time(&mut self, t: DateTime<FixedOffset>) -> ZKResult<()>`
Sets the device time to the specified timestamp.

### `pub fn get_users(&mut self) -> ZKResult<Vec<User>>`
Retrieves all users from the device.

### `pub fn set_user(&mut self, user: &User) -> ZKResult<()>`
Creates or updates a user on the device. Automatically validates that the User ID is unique across all UIDs to prevent data conflicts.

### `pub fn delete_user(&mut self, uid: u16) -> ZKResult<()>`
Deletes a specific user from the device by their internal UID.

### `pub fn find_user_by_id(&mut self, user_id: &str) -> ZKResult<Option<User>>`
Finds a user on the device by their alphanumeric User ID string.

### `pub fn get_next_free_uid(&mut self, start_uid: u16) -> ZKResult<u16>`
Helper to find the next available internal index (UID) on the device, starting from `start_uid`.

### `pub fn refresh_data(&mut self) -> ZKResult<()>`
Triggers a data refresh on the device, ensuring all recent modifications (like user additions) are finalized.

### `pub fn read_sizes(&mut self) -> ZKResult<()>`
Updates device capacity and usage info.

### `pub fn unlock(&mut self, seconds: u32) -> ZKResult<()>`
Sends an unlock command to the device.
