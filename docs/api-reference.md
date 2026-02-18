# API Reference

## ZK Struct

### `pub fn new(addr: &str, port: u16) -> Self`
Creates a new ZK instance.

### `pub fn connect(&mut self, tcp: bool) -> ZKResult<()>`
Connects to the device using TCP or UDP.

### `pub fn get_attendance(&mut self) -> ZKResult<Vec<Attendance>>`
Retrieves all attendance logs from the device.

### `pub fn get_users(&mut self) -> ZKResult<Vec<User>>`
Retrieves all users from the device.

### `pub fn read_sizes(&mut self) -> ZKResult<()>`
Updates device capacity and usage info.

### `pub fn unlock(&mut self, seconds: u32) -> ZKResult<()>`
Sends an unlock command to the device.
