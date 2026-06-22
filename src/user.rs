use byteorder::{ByteOrder, LittleEndian};
use std::collections::HashMap;

use crate::constants::*;
use crate::models::{Finger, User};
use crate::{ZKError, ZKErrorCode, ZKResult, ZK};

impl ZK {
    /// Retrieves all users from the device.
    pub fn get_users(&mut self) -> ZKResult<Vec<User>> {
        self.read_sizes()?;
        if self.users == 0 {
            return Ok(Vec::new());
        }

        let userdata = self.read_with_buffer(CMD_USERTEMP_RRQ, FCT_USER as u32, 0)?;
        if userdata.len() <= 4 {
            return Ok(Vec::new());
        }

        let total_size = LittleEndian::read_u32(&userdata[0..4]) as usize;
        if total_size > MAX_RESPONSE_SIZE {
            return Err(ZKError::InvalidData(
                ZKErrorCode::BufferOverflow,
                format!(
                    "User data total_size {} exceeds maximum {}",
                    total_size, MAX_RESPONSE_SIZE
                ),
            ));
        }
        if total_size == 0 {
            return Ok(Vec::new());
        }
        self.user_packet_size = total_size / self.users as usize;
        let data = &userdata[4..];

        let mut users = Vec::with_capacity(std::cmp::min(
            self.users as usize,
            data.len() / self.user_packet_size,
        ));
        let mut offset = 0;

        if self.user_packet_size == USER_PACKET_SIZE_SMALL {
            while offset + USER_PACKET_SIZE_SMALL <= data.len() {
                let chunk = &data[offset..offset + USER_PACKET_SIZE_SMALL];
                let user = User::parse_small(chunk)?;
                users.push(user);
                offset += USER_PACKET_SIZE_SMALL;
            }
        } else if self.user_packet_size == USER_PACKET_SIZE_LARGE {
            while offset + USER_PACKET_SIZE_LARGE <= data.len() {
                let chunk = &data[offset..offset + USER_PACKET_SIZE_LARGE];
                let user = User::parse_large(chunk)?;
                users.push(user);
                offset += USER_PACKET_SIZE_LARGE;
            }
        } else {
            return Err(ZKError::Response(
                ZKErrorCode::ProtocolViolation,
                format!(
                    "Unsupported user packet size: {}. Device might be using an unknown protocol version.",
                    self.user_packet_size
                ),
            ));
        }

        Ok(users)
    }

    /// Creates or updates a user on the device.
    /// Ensures User ID uniqueness to prevent logic conflicts.
    /// **Performance Note:** This performs an O(N) fetch of all users first. For bulk operations, use `set_user_unchecked`.
    pub fn set_user(&mut self, user: &User) -> ZKResult<()> {
        // 1. Ensure cache is loaded
        if self.user_id_cache.is_none() {
            self.refresh_user_cache()?;
        }

        // 2. Safety Check: Ensure this User ID doesn't already exist under a DIFFERENT UID.
        // If it exists under the SAME UID, it's an update, which is allowed.
        if let Some(ref cache) = self.user_id_cache {
            if let Some((&existing_uid, _)) = cache
                .iter()
                .find(|(&uid, id)| *id == user.user_id() && uid != user.uid())
            {
                return Err(ZKError::Response(
                    ZKErrorCode::DataConflict,
                    format!(
                        "Conflict: User ID '{}' already exists on the device at UID {}",
                        user.user_id(),
                        existing_uid
                    ),
                ));
            }
        }

        self.set_user_unchecked(user)
    }

    /// Creates or updates multiple users on the device in a single operation.
    /// This is highly efficient as it fetches the user list and refreshes data only once.
    /// Performs safety checks for User ID uniqueness across the batch and existing users.
    pub fn set_users_bulk(&mut self, users: &[User]) -> ZKResult<()> {
        if users.is_empty() {
            return Ok(());
        }

        // 1. Ensure cache is loaded
        if self.user_id_cache.is_none() {
            self.refresh_user_cache()?;
        }

        // 2. Perform uniqueness checks for the entire batch using local cache
        if let Some(ref mut cache) = self.user_id_cache {
            for user in users {
                if let Some((&existing_uid, _)) = cache
                    .iter()
                    .find(|(&uid, id)| *id == user.user_id() && uid != user.uid())
                {
                    return Err(ZKError::Response(
                        ZKErrorCode::DataConflict,
                        format!(
                            "Conflict in batch: User ID '{}' already exists on device at UID {}",
                            user.user_id(),
                            existing_uid
                        ),
                    ));
                }

                // Update the local cache to reflect the state for subsequent users in the batch
                cache.retain(|&uid, id| uid == user.uid() || id != user.user_id());
                cache.insert(user.uid(), user.user_id().to_string());
            }
        }

        // 3. Send all users without individual refreshes
        for user in users {
            self.set_user_unchecked_no_refresh(user)?;
        }

        // 4. Refresh device data once at the end
        let _ = self.refresh_data();
        Ok(())
    }

    /// Creates or updates a user on the device WITHOUT uniqueness checks.
    /// High performance, suitable for bulk syncing.
    pub fn set_user_unchecked(&mut self, user: &User) -> ZKResult<()> {
        self.set_user_unchecked_no_refresh(user)?;
        let _ = self.refresh_data();
        Ok(())
    }

    /// Internal helper to set a user without sending a REFRESHDATA command.
    fn set_user_unchecked_no_refresh(&mut self, user: &User) -> ZKResult<()> {
        let payload = if self.user_packet_size == USER_PACKET_SIZE_SMALL {
            user.to_bytes_small()?
        } else {
            user.to_bytes_large()?
        };

        let res = self.send_command(CMD_USER_WRQ, &payload)?;
        if res.command() == CMD_ACK_OK {
            // Update the local cache to match the new mapping
            if let Some(ref mut cache) = self.user_id_cache {
                cache.retain(|&uid, id| uid == user.uid() || id != user.user_id());
                cache.insert(user.uid(), user.user_id().to_string());
            }
            Ok(())
        } else {
            Err(ZKError::Response(
                ZKErrorCode::ProtocolViolation,
                "Failed to set user".into(),
            ))
        }
    }

    /// Deletes a specific user by UID.
    pub fn delete_user(&mut self, uid: u16) -> ZKResult<()> {
        let mut payload = [0u8; 2];
        LittleEndian::write_u16(&mut payload, uid);

        let res = self.send_command(CMD_DELETE_USER, &payload)?;
        if res.command() == CMD_ACK_OK {
            // Remove the deleted user mapping from cache
            if let Some(ref mut cache) = self.user_id_cache {
                cache.remove(&uid);
            }
            let _ = self.refresh_data();
            Ok(())
        } else {
            Err(ZKError::Response(
                ZKErrorCode::ProtocolViolation,
                "Failed to delete user".into(),
            ))
        }
    }

    /// Retrieves all fingerprint templates from the device.
    pub fn get_templates(&mut self) -> ZKResult<Vec<Finger>> {
        self.read_sizes()?;
        if self.fingers == 0 {
            return Ok(Vec::new());
        }

        let templatedata = self.read_with_buffer(CMD_DB_RRQ, FCT_FINGERTMP as u32, 0)?;
        if templatedata.len() < 4 {
            return Ok(Vec::new());
        }

        let raw_total = LittleEndian::read_i32(&templatedata[0..4]);
        if raw_total < 0 {
            return Err(ZKError::InvalidData(
                ZKErrorCode::InvalidDataFormat,
                format!("Negative template data size: {}", raw_total),
            ));
        }
        let mut total_size = raw_total as usize;
        if total_size > MAX_RESPONSE_SIZE {
            return Err(ZKError::InvalidData(
                ZKErrorCode::BufferOverflow,
                format!(
                    "Template data size {} exceeds maximum {}",
                    total_size, MAX_RESPONSE_SIZE
                ),
            ));
        }
        let mut data = &templatedata[4..];
        let mut templates =
            Vec::with_capacity(std::cmp::min(self.fingers as usize, data.len() / 6));

        while total_size > 0 && data.len() >= 6 {
            let size = LittleEndian::read_u16(&data[0..2]) as usize;
            let uid = LittleEndian::read_u16(&data[2..4]);
            let fid = data[4];
            let valid = data[5];

            if data.len() < size {
                break;
            }

            let template = data[6..size].to_vec();
            templates.push(Finger::new(uid, fid, valid, template));

            data = &data[size..];
            if total_size >= size {
                total_size -= size;
            } else {
                total_size = 0;
            }
        }

        Ok(templates)
    }

    /// Retrieves a specific fingerprint template for a user and finger ID.
    pub fn get_user_template(&mut self, uid: u16, fid: u8) -> ZKResult<Option<Finger>> {
        for _ in 0..3 {
            let mut payload = [0u8; 3];
            LittleEndian::write_u16(&mut payload[0..2], uid);
            payload[2] = fid;

            let res = self.send_command(_CMD_GET_USERTEMP, &payload)?;
            // This command typically returns CMD_DATA with the template
            if res.command() == CMD_DATA {
                let mut template = res.into_payload().into_owned();
                // Strip trailing nulls if present (common firmware quirk)
                while template.ends_with(&[0]) && !template.is_empty() {
                    template.pop();
                }
                return Ok(Some(Finger::new(uid, fid, 1, template)));
            }
        }
        Ok(None)
    }

    /// Deletes a specific fingerprint template for a user and finger ID.
    pub fn delete_user_template(&mut self, uid: u16, fid: u8) -> ZKResult<()> {
        let mut payload = [0u8; 3];
        LittleEndian::write_u16(&mut payload[0..2], uid);
        payload[2] = fid;

        let res = self.send_command(CMD_DELETE_USERTEMP, &payload)?;
        if res.command() == CMD_ACK_OK {
            let _ = self.refresh_data();
            Ok(())
        } else {
            Err(ZKError::Response(
                ZKErrorCode::ProtocolViolation,
                "Failed to delete user template".into(),
            ))
        }
    }

    /// Finds a user on the device by their alphanumeric User ID.
    pub fn find_user_by_id(&mut self, user_id: &str) -> ZKResult<Option<User>> {
        let users = self.get_users()?;
        Ok(users.into_iter().find(|u| u.user_id() == user_id))
    }

    /// Explicitly refreshes the internal user ID cache.
    pub fn refresh_user_cache(&mut self) -> ZKResult<()> {
        let users = self.get_users()?;
        let mut cache = HashMap::with_capacity(users.len());
        for user in users {
            cache.insert(user.uid(), user.user_id().to_string());
        }
        self.user_id_cache = Some(cache);
        Ok(())
    }

    /// Internal helper to get User ID for a UID, using cache if available.
    pub(crate) fn get_user_id_from_cache(&mut self, uid: u16) -> String {
        if self.user_id_cache.is_none() {
            if let Err(e) = self.refresh_user_cache() {
                log::warn!(
                    "Failed to refresh user cache: {}. Falling back to default mappings.",
                    e
                );
                // Initialize cache to empty map to prevent infinite retry loops on network timeout
                self.user_id_cache = Some(HashMap::new());
            }
        }

        self.user_id_cache
            .as_ref()
            .and_then(|c| c.get(&uid).cloned())
            .unwrap_or_else(|| uid.to_string())
    }
}
