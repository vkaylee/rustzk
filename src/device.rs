use byteorder::{ByteOrder, LittleEndian};
use chrono::{DateTime, FixedOffset, TimeZone};

use crate::constants::*;
use crate::{ZKError, ZKErrorCode, ZKResult, ZK};

impl ZK {
    /// Fetches device capacity and usage statistics.
    pub fn read_sizes(&mut self) -> ZKResult<()> {
        let mut res = self.send_command(CMD_GET_FREE_SIZES, &[])?;

        // Handle case where device sends ACK_OK then ACK_DATA/Response separately
        if res.command() == CMD_ACK_OK && res.payload().len() < 16 {
            // Try reading the next packet which should contain the actual data
            match self.read_response_safe() {
                Ok(next_packet) => {
                    res = next_packet;
                }
                Err(e) => {
                    log::debug!(
                        "read_sizes: received ACK_OK but failed to read subsequent data: {}",
                        e
                    );
                }
            }
        }

        if res.command() == CMD_ACK_OK || res.command() == CMD_ACK_DATA {
            let data = res.payload();
            if data.len() >= 80 {
                self.users = LittleEndian::read_i32(&data[16..20]).max(0) as u32;
                self.fingers = LittleEndian::read_i32(&data[24..28]).max(0) as u32;
                self.records = LittleEndian::read_i32(&data[32..36]).max(0) as u32;
                self.cards = LittleEndian::read_i32(&data[48..52]);
                self.fingers_cap = LittleEndian::read_i32(&data[56..60]);
                self.users_cap = LittleEndian::read_i32(&data[60..64]);
                self.rec_cap = LittleEndian::read_i32(&data[64..68]);

                if data.len() >= 92 {
                    self.faces = LittleEndian::read_i32(&data[80..84]).max(0) as u32;
                    self.faces_cap = LittleEndian::read_i32(&data[88..92]);
                }
            } else if data.len() >= 28 {
                // Older firmware formats
                self.users = LittleEndian::read_i32(&data[0..4]).max(0) as u32;
                self.fingers = LittleEndian::read_i32(&data[4..8]).max(0) as u32;
                self.records = LittleEndian::read_i32(&data[8..12]).max(0) as u32;
                self.users_cap = LittleEndian::read_i32(&data[12..16]);
                self.fingers_cap = LittleEndian::read_i32(&data[16..20]);
                self.rec_cap = LittleEndian::read_i32(&data[20..24]);
            }
            let _ = self.sync_timezone();
            Ok(())
        } else {
            Err(ZKError::Response(
                ZKErrorCode::ProtocolViolation,
                format!(
                    "Failed to read sizes (Device returned CMD 0x{:X})",
                    res.command()
                ),
            ))
        }
    }

    pub fn get_firmware_version(&mut self) -> ZKResult<String> {
        let res = self.send_command(CMD_GET_VERSION, &[])?;
        if res.command() == CMD_ACK_OK || res.command() == CMD_ACK_DATA {
            Ok(String::from_utf8_lossy(res.payload())
                .trim_matches('\0')
                .to_string())
        } else {
            Err(ZKError::Response(
                ZKErrorCode::ProtocolViolation,
                "Can't read firmware version".into(),
            ))
        }
    }

    pub fn get_option_value(&mut self, key: &str) -> ZKResult<String> {
        let mut command_string = key.as_bytes().to_vec();
        command_string.push(0);
        let res = self.send_command(CMD_OPTIONS_RRQ, &command_string)?;
        if res.command() == CMD_ACK_OK || res.command() == CMD_ACK_DATA {
            let data = String::from_utf8_lossy(res.payload());
            let data_str = data.trim_matches('\0').to_string();

            // Usually returns "Key=Value"
            if let Some(pos) = data_str.find('=') {
                Ok(data_str[pos + 1..].to_string())
            } else {
                Ok(data_str)
            }
        } else {
            Err(ZKError::Response(
                ZKErrorCode::ProtocolViolation,
                format!(
                    "Can't read option '{}' (Device returned CMD 0x{:X})",
                    key,
                    res.command()
                ),
            ))
        }
    }

    pub fn get_serial_number(&mut self) -> ZKResult<String> {
        self.get_option_value("~SerialNumber")
    }

    pub fn get_platform(&mut self) -> ZKResult<String> {
        self.get_option_value("~Platform")
    }

    /// Gets the device timezone adjustment (usually in hours).
    pub fn get_timezone(&mut self) -> ZKResult<i32> {
        let tz_str = self.get_option_value("TZAdj")?;
        tz_str.parse::<i32>().map_err(|_| {
            ZKError::InvalidData(
                ZKErrorCode::InvalidDataFormat,
                format!("Invalid timezone value: {}", tz_str),
            )
        })
    }

    pub fn get_mac(&mut self) -> ZKResult<String> {
        self.get_option_value("MAC")
    }

    pub fn get_device_name(&mut self) -> ZKResult<String> {
        self.get_option_value("~DeviceName")
    }

    pub fn get_face_version(&mut self) -> ZKResult<String> {
        self.get_option_value("ZKFaceVersion")
    }

    pub fn get_fp_version(&mut self) -> ZKResult<String> {
        self.get_option_value("~ZKFPVersion")
    }

    /// Retrieves the current time from the device.
    ///
    /// **Note:** This returns the time as configured on the device, mapped to
    /// the detected timezone offset. If the device time falls into a DST
    /// transition gap (non-existent time), this will return an error.
    pub fn get_time(&mut self) -> ZKResult<DateTime<FixedOffset>> {
        let _ = self.sync_timezone();
        let res = self.send_command(CMD_GET_TIME, &[])?;
        if res.command() == CMD_ACK_OK || res.command() == CMD_ACK_DATA {
            let naive = ZK::decode_time(res.payload())?;
            let offset = FixedOffset::east_opt(self.timezone_offset * 60).unwrap_or_else(|| {
                match FixedOffset::east_opt(0) {
                    Some(o) => o,
                    None => unreachable!(),
                }
            });

            match offset.from_local_datetime(&naive) {
                chrono::LocalResult::Single(dt) => Ok(dt),
                chrono::LocalResult::Ambiguous(dt1, _) => Ok(dt1),
                chrono::LocalResult::None => Err(ZKError::InvalidData(
                    ZKErrorCode::InvalidDataFormat,
                    "Invalid local datetime from device".into(),
                )),
            }
        } else {
            Err(ZKError::Response(
                ZKErrorCode::ProtocolViolation,
                "Can't get time".into(),
            ))
        }
    }

    pub fn set_time(&mut self, t: DateTime<FixedOffset>) -> ZKResult<()> {
        // ZKTeco devices usually work in local time.
        let local_naive = t.naive_local();
        let encoded = ZK::encode_time(local_naive);
        let mut payload = [0u8; 4];
        byteorder::LittleEndian::write_u32(&mut payload, encoded);

        let res = self.send_command(CMD_SET_TIME, &payload)?;
        if res.command() == CMD_ACK_OK {
            Ok(())
        } else {
            Err(ZKError::Response(
                ZKErrorCode::ProtocolViolation,
                "Failed to set time".into(),
            ))
        }
    }

    /// Sets a device option by key and value.
    pub fn set_option(&mut self, key: &str, value: &str) -> ZKResult<()> {
        let command_string = format!("{}={}\0", key, value);
        let res = self.send_command(CMD_OPTIONS_WRQ, command_string.as_bytes())?;
        if res.command() == CMD_ACK_OK {
            Ok(())
        } else {
            Err(ZKError::Response(
                ZKErrorCode::ProtocolViolation,
                format!("Failed to set option {}", key),
            ))
        }
    }

    /// Changes the communication password (CommKey) of the device.
    /// Note: After changing the password, you must use the new password for future connections.
    pub fn change_password(&mut self, new_password: u32) -> ZKResult<()> {
        self.set_option("ComKey", &new_password.to_string())?;
        self.password = new_password; // Update local state to match
        Ok(())
    }

    pub fn restart(&mut self) -> ZKResult<()> {
        let result = self.send_command(CMD_RESTART, &[]);
        self.is_connected = false;
        self.transport = None;
        result.map(|_| ())
    }

    pub fn poweroff(&mut self) -> ZKResult<()> {
        let result = self.send_command(CMD_POWEROFF, &[]);
        self.is_connected = false;
        self.transport = None;
        result.map(|_| ())
    }

    pub fn unlock(&mut self, seconds: u32) -> ZKResult<()> {
        let mut payload = [0u8; 4];
        byteorder::LittleEndian::write_u32(&mut payload, seconds * 10);
        let res = self.send_command(CMD_UNLOCK, &payload)?;
        if res.command() == CMD_ACK_OK {
            Ok(())
        } else {
            Err(ZKError::Response(
                ZKErrorCode::ProtocolViolation,
                "Can't open door".into(),
            ))
        }
    }

    /// Refreshes the device's internal data.
    pub fn refresh_data(&mut self) -> ZKResult<()> {
        let res = self.send_command(CMD_REFRESHDATA, &[])?;
        if res.command() == CMD_ACK_OK {
            Ok(())
        } else {
            Err(ZKError::Response(
                ZKErrorCode::ProtocolViolation,
                "Failed to refresh data".into(),
            ))
        }
    }

    /// Returns the detected timezone offset in minutes.
    pub fn timezone_offset(&self) -> i32 {
        self.timezone_offset
    }

    /// Returns true if the timezone has been synchronized with the device.
    pub fn timezone_synced(&self) -> bool {
        self.timezone_synced
    }

    pub fn users(&self) -> u32 {
        self.users
    }
    pub fn users_cap(&self) -> i32 {
        self.users_cap
    }
    pub fn fingers(&self) -> u32 {
        self.fingers
    }
    pub fn fingers_cap(&self) -> i32 {
        self.fingers_cap
    }
    pub fn records(&self) -> u32 {
        self.records
    }
    pub fn records_cap(&self) -> i32 {
        self.rec_cap
    }
    pub fn faces(&self) -> u32 {
        self.faces
    }
    pub fn faces_cap(&self) -> i32 {
        self.faces_cap
    }
    pub fn cards(&self) -> i32 {
        self.cards
    }
    pub fn is_connected(&self) -> bool {
        self.is_connected
    }
}
