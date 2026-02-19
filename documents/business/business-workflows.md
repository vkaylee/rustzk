# Business Process Flows: rustzk

## 1. Onboarding Employee (Create User)
1. **Request:** HR adds a new employee in the payroll system.
2. **Process:**
   - Payroll system calls `rustzk::ZK::set_user()` with Name, ID, Card Number.
   - Payroll system calls `rustzk::ZK::set_fingerprint()` (optional).
   - `rustzk` sends `CMD_USER_WRQ` to the device.
3. **Outcome:** User is created on the device. Employee can now clock in.

## 2. Daily Attendance Sync
1. **Trigger:** Scheduled job (e.g., every 15 minutes).
2. **Process:**
   - Sync service connects to `rustzk`.
   - Calls `rustzk::get_attendance()`.
   - Iterates through returned `Attendance` logs.
   - Saves logs to central database.
   - (Optional) Calls `rustzk::clear_attendance()` to free device memory.
3. **Outcome:** Central database has up-to-date attendance records.

## 3. Offboarding Employee (Terminate User)
1. **Trigger:** Employee termination date reached.
2. **Process:**
   - HR system calls `rustzk::delete_user(uid)`.
   - `rustzk` sends `CMD_DELETE_USER` to the device.
   - `rustzk` verifies deletion with `ACK_OK`.
3. **Outcome:** User cannot access the premises or clock in.

## 4. Device Maintenance
1. **Trigger:** Admin initiates maintenance window.
2. **Process:**
   - Admin connects via `rustzk`.
   - Calls `get_firmware_version()` to check for updates.
   - Calls `get_time()` to verify clock accuracy.
   - Calls `set_time()` if drift > 60 seconds.
   - Calls `restart()` to apply pending changes.
3. **Outcome:** Device remains healthy and synchronized.
