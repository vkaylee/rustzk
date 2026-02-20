# API Reference

## ZK Struct

### `pub fn new(addr: &str, port: u16) -> Self`
Creates a new ZK instance.

### `pub fn set_password(&mut self, password: u32)`
Sets the password for authentication. Default is 0.

### `pub fn connect(&mut self, protocol: ZKProtocol) -> ZKResult<()>`
Connects to the device using the specified protocol.

**`ZKProtocol` Variants:**
- `TCP`: Force TCP connection.
- `UDP`: Force UDP connection.
- `Auto`: Attempt TCP first, fallback to UDP.

### `pub fn get_attendance(&mut self) -> ZKResult<Vec<Attendance>>`
Retrieves all attendance logs from the device.

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
