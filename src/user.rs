use std::collections::HashMap;
use std::io::{self, Read, Write};
use byteorder::{ByteOrder, LittleEndian, ReadBytesExt, WriteBytesExt};

use crate::{ZK, ZKError, ZKResult};
use crate::models::{User, Finger};
use crate::constants::*;

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
            return Err(ZKError::InvalidData(format!(
                "User data total_size {} exceeds maximum {}",
                total_size, MAX_RESPONSE_SIZE
            )));
        }
        if total_size == 0 {
            return Ok(Vec::new());
        }
        self.user_packet_size = total_size / self.users as usize;
        let data = &userdata[4..];

        let mut users = Vec::with_capacity(self.users as usize);
        let mut offset = 0;

        if self.user_packet_size == USER_PACKET_SIZE_SMALL {
            while offset + USER_PACKET_SIZE_SMALL <= data.len() {
                let chunk = &data[offset..offset + USER_PACKET_SIZE_SMALL];
                let mut rdr = io::Cursor::new(chunk);
                let uid = rdr.read_u16::<byteorder::LittleEndian>()?;
                let privilege = rdr.read_u8()?;
                let mut password_bytes = [0u8; 5];
                rdr.read_exact(&mut password_bytes)?;
                let mut name_bytes = [0u8; 8];
                rdr.read_exact(&mut name_bytes)?;
                let card = rdr.read_u32::<byteorder::LittleEndian>()?;
                let _pad = rdr.read_u8()?;
                let group_id = rdr.read_u8()?;
                let _timezone = rdr.read_u16::<byteorder::LittleEndian>()?;
                let user_id = rdr.read_u32::<byteorder::LittleEndian>()?;

                users.push(User::new(
                    uid,
                    ZK::decode_gbk(&name_bytes),
                    privilege,
                    String::from_utf8_lossy(&password_bytes)
                        .trim_matches('\0')
                        .to_string(),
                    group_id.to_string(),
                    user_id.to_string(),
                    card,
                ));
                offset += USER_PACKET_SIZE_SMALL;
            }
        } else if self.user_packet_size == USER_PACKET_SIZE_LARGE {
            while offset + USER_PACKET_SIZE_LARGE <= data.len() {
                let chunk = &data[offset..offset + USER_PACKET_SIZE_LARGE];
                let mut rdr = io::Cursor::new(chunk);
                let uid = rdr.read_u16::<byteorder::LittleEndian>()?;
                let privilege = rdr.read_u8()?;
                let mut password_bytes = [0u8; 8];
                rdr.read_exact(&mut password_bytes)?;
                let mut name_bytes = [0u8; 24];
                rdr.read_exact(&mut name_bytes)?;
                let card = rdr.read_u32::<byteorder::LittleEndian>()?;
                let _pad1 = rdr.read_u8()?;
                let mut group_id_bytes = [0u8; 7];
                rdr.read_exact(&mut group_id_bytes)?;
                let _pad2 = rdr.read_u8()?;
                let mut user_id_bytes = [0u8; 24];
                rdr.read_exact(&mut user_id_bytes)?;

                users.push(User::new(
                    uid,
                    ZK::decode_gbk(&name_bytes),
                    privilege,
                    String::from_utf8_lossy(&password_bytes)
                        .trim_matches('\0')
                        .to_string(),
                    String::from_utf8_lossy(&group_id_bytes)
                        .trim_matches('\0')
                        .to_string(),
                    String::from_utf8_lossy(&user_id_bytes)
                        .trim_matches('\0')
                        .to_string(),
                    card,
                ));
                offset += USER_PACKET_SIZE_LARGE;
            }
        } else {
            return Err(ZKError::Response(format!(
                "Unsupported user packet size: {}. Device might be using an unknown protocol version.",
                self.user_packet_size
            )));
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
            if let Some((&existing_uid, _)) = cache.iter().find(|(&uid, id)| *id == user.user_id() && uid != user.uid()) {
                return Err(ZKError::Response(format!(
                    "Conflict: User ID '{}' already exists on the device at UID {}",
                    user.user_id(), existing_uid
                )));
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
                if let Some((&existing_uid, _)) = cache.iter().find(|(&uid, id)| *id == user.user_id() && uid != user.uid()) {
                    return Err(ZKError::Response(format!(
                        "Conflict in batch: User ID '{}' already exists on device at UID {}",
                        user.user_id(), existing_uid
                    )));
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
        let mut payload = Vec::new();

        if self.user_packet_size == 28 {
            payload.write_u16::<LittleEndian>(user.uid())?;
            payload.write_u8(user.privilege())?;

            let mut password_bytes = [0u8; 5];
            let p_bytes = user.password().as_bytes();
            let p_len = std::cmp::min(p_bytes.len(), 5);
            password_bytes[..p_len].copy_from_slice(&p_bytes[..p_len]);
            payload.write_all(&password_bytes)?;

            let mut name_bytes = [0u8; 8];
            let n_bytes_gbk = encoding_rs::GBK.encode(user.name()).0;
            let n_len = std::cmp::min(n_bytes_gbk.len(), 8);
            name_bytes[..n_len].copy_from_slice(&n_bytes_gbk[..n_len]);
            payload.write_all(&name_bytes)?;

            payload.write_u32::<LittleEndian>(user.card())?;
            payload.write_u8(0)?; // pad
            let group_id = user.group_id().parse::<u8>().unwrap_or(0);
            payload.write_u8(group_id)?;
            payload.write_u16::<LittleEndian>(0)?; // timezone/pad
            let user_id_num = user.user_id().parse::<u32>().unwrap_or(0);
            payload.write_u32::<LittleEndian>(user_id_num)?;
        } else {
            // 72-byte format
            payload.write_u16::<LittleEndian>(user.uid())?;
            payload.write_u8(user.privilege())?;

            let mut password_bytes = [0u8; 8];
            let p_bytes = user.password().as_bytes();
            let p_len = std::cmp::min(p_bytes.len(), 8);
            password_bytes[..p_len].copy_from_slice(&p_bytes[..p_len]);
            payload.write_all(&password_bytes)?;

            let mut name_bytes = [0u8; 24];
            let n_bytes_gbk = encoding_rs::GBK.encode(user.name()).0;
            let n_len = std::cmp::min(n_bytes_gbk.len(), 24);
            name_bytes[..n_len].copy_from_slice(&n_bytes_gbk[..n_len]);
            payload.write_all(&name_bytes)?;

            payload.write_u32::<LittleEndian>(user.card())?;
            payload.write_u8(0)?; // pad1

            let mut group_id_bytes = [0u8; 7];
            let g_bytes = user.group_id().as_bytes();
            let g_len = std::cmp::min(g_bytes.len(), 7);
            group_id_bytes[..g_len].copy_from_slice(&g_bytes[..g_len]);
            payload.write_all(&group_id_bytes)?;

            payload.write_u8(0)?; // pad2

            let mut user_id_bytes = [0u8; 24];
            let u_bytes = user.user_id().as_bytes();
            let u_len = std::cmp::min(u_bytes.len(), 24);
            user_id_bytes[..u_len].copy_from_slice(&u_bytes[..u_len]);
            payload.write_all(&user_id_bytes)?;
        }

        let res = self.send_command(CMD_USER_WRQ, &payload)?;
        if res.command() == CMD_ACK_OK {
            // Update the local cache to match the new mapping
            if let Some(ref mut cache) = self.user_id_cache {
                cache.retain(|&uid, id| uid == user.uid() || id != user.user_id());
                cache.insert(user.uid(), user.user_id().to_string());
            }
            Ok(())
        } else {
            Err(ZKError::Response("Failed to set user".into()))
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
            Err(ZKError::Response("Failed to delete user".into()))
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
            return Err(ZKError::InvalidData(format!(
                "Negative template data size: {}",
                raw_total
            )));
        }
        let mut total_size = raw_total as usize;
        if total_size > MAX_RESPONSE_SIZE {
            return Err(ZKError::InvalidData(format!(
                "Template data size {} exceeds maximum {}",
                total_size, MAX_RESPONSE_SIZE
            )));
        }
        let mut data = &templatedata[4..];
        let mut templates = Vec::with_capacity(self.fingers as usize);

        while total_size > 0 && data.len() >= 6 {
            let size = LittleEndian::read_u16(&data[0..2]) as usize;
            let uid = LittleEndian::read_u16(&data[2..4]);
            let fid = data[4];
            let valid = data[5];

            if data.len() < size {
                break;
            }

            let template = data[6..size].to_vec();
            templates.push(Finger::new(
                uid,
                fid,
                valid,
                template,
            ));

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
                return Ok(Some(Finger::new(
                    uid,
                    fid,
                    1,
                    template,
                )));
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
            Err(ZKError::Response("Failed to delete user template".into()))
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
                log::warn!("Failed to refresh user cache: {}. Falling back to default mappings.", e);
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
