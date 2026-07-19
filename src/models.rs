use crate::constants::{USER_PACKET_SIZE_LARGE, USER_PACKET_SIZE_SMALL};
use crate::{ZKError, ZKResult};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use chrono::{DateTime, FixedOffset, NaiveDateTime, TimeZone, Utc};
use std::io::{Cursor, Read, Write};

/// Represents an attendance record (clock-in/out).
#[derive(Debug, Clone)]
pub struct Attendance {
    /// Internal record UID (sequence number).
    uid: u32,
    /// The user ID string associated with the record.
    user_id: String,
    /// The raw timestamp from the device.
    timestamp: NaiveDateTime,
    /// Attendance status code.
    status: u8,
    /// Punch type (e.g., finger, face, card).
    punch: u8,
    /// The timezone offset in minutes applied to this record.
    timezone_offset: i32,
}

impl Attendance {
    /// Creates a new Attendance record.
    pub fn new(
        uid: u32,
        user_id: String,
        timestamp: NaiveDateTime,
        status: u8,
        punch: u8,
        timezone_offset: i32,
    ) -> Self {
        Self {
            uid,
            user_id,
            timestamp,
            status,
            punch,
            timezone_offset,
        }
    }

    /// Getter for uid.
    pub fn uid(&self) -> u32 {
        self.uid
    }

    /// Getter for user_id.
    pub fn user_id(&self) -> &str {
        &self.user_id
    }

    /// Getter for timestamp.
    pub fn timestamp(&self) -> NaiveDateTime {
        self.timestamp
    }

    /// Getter for status.
    pub fn status(&self) -> u8 {
        self.status
    }

    /// Getter for punch.
    pub fn punch(&self) -> u8 {
        self.punch
    }

    /// Returns the timestamp as a DateTime with the device's fixed offset.
    ///
    /// **Note:** This method attempts to map the raw local time from the device
    /// to a specific offset. It may return `None` if the time is invalid or
    /// ambiguous (e.g., during DST transitions).
    ///
    /// For critical operations, prefer using the raw `.timestamp` (NaiveDateTime)
    /// and handle timezones at the application level.
    pub fn timestamp_fixed(&self) -> Option<DateTime<FixedOffset>> {
        // Sanity check: limit offset to +/- 24 hours (1440 minutes)
        if self.timezone_offset.abs() > 1440 {
            return None;
        }
        let offset = FixedOffset::east_opt(self.timezone_offset * 60)
            .or_else(|| FixedOffset::east_opt(0))?;
        match offset.from_local_datetime(&self.timestamp) {
            chrono::LocalResult::Single(dt) => Some(dt),
            chrono::LocalResult::Ambiguous(dt1, _) => Some(dt1),
            chrono::LocalResult::None => None,
        }
    }

    /// Returns the timestamp in UTC.
    pub fn timestamp_utc(&self) -> Option<DateTime<Utc>> {
        self.timestamp_fixed()
            .map(|fixed| fixed.with_timezone(&Utc))
    }

    /// Returns the timestamp formatted as an ISO8601 string with offset.
    /// Returns a naive ISO8601 string if the offset mapping fails.
    pub fn iso_format(&self) -> String {
        match self.timestamp_fixed() {
            Some(fixed) => fixed.to_rfc3339(),
            None => self.timestamp.format("%Y-%m-%dT%H:%M:%S").to_string(),
        }
    }

    /// Returns the timezone offset in minutes applied to this record.
    pub fn timezone_offset(&self) -> i32 {
        self.timezone_offset
    }
}

/// Represents a user on the ZK device.
#[derive(Debug, Clone)]
pub struct User {
    /// Internal user UID.
    uid: u16,
    /// User's display name.
    name: String,
    /// User's privilege level (Admin, User, etc.).
    privilege: u8,
    /// User's numeric password (if any).
    password: String,
    /// ID of the group the user belongs to.
    group_id: String,
    /// The alphanumeric user ID string.
    user_id: String,
    /// ID of the proximity card assigned to the user.
    card: u32,
}

impl User {
    /// Creates a new User record.
    pub fn new(
        uid: u16,
        name: String,
        privilege: u8,
        password: String,
        group_id: String,
        user_id: String,
        card: u32,
    ) -> Self {
        Self {
            uid,
            name,
            privilege,
            password,
            group_id,
            user_id,
            card,
        }
    }

    /// Getter for uid.
    pub fn uid(&self) -> u16 {
        self.uid
    }

    /// Getter for name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Setter for name.
    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    /// Getter for privilege.
    pub fn privilege(&self) -> u8 {
        self.privilege
    }

    /// Getter for password.
    pub fn password(&self) -> &str {
        &self.password
    }

    /// Getter for group_id.
    pub fn group_id(&self) -> &str {
        &self.group_id
    }

    /// Getter for user_id.
    pub fn user_id(&self) -> &str {
        &self.user_id
    }

    /// Getter for card.
    pub fn card(&self) -> u32 {
        self.card
    }

    /// Returns true if the user is disabled.
    pub fn is_disabled(&self) -> bool {
        (self.privilege & 1) != 0
    }

    /// Returns true if the user is enabled.
    pub fn is_enabled(&self) -> bool {
        !self.is_disabled()
    }

    /// Returns the raw user type bits.
    pub fn user_type(&self) -> u8 {
        self.privilege & 0xE
    }

    /// Parses a User from a 28-byte raw chunk.
    pub fn parse_small(chunk: &[u8]) -> ZKResult<Self> {
        parse_user::<SmallLayout>(chunk)
    }

    /// Parses a User from a 72-byte raw chunk.
    pub fn parse_large(chunk: &[u8]) -> ZKResult<Self> {
        parse_user::<LargeLayout>(chunk)
    }

    /// Serializes the user into a 28-byte raw vector.
    pub fn to_bytes_small(&self) -> ZKResult<Vec<u8>> {
        user_to_bytes::<SmallLayout>(self)
    }

    /// Serializes the user into a 72-byte raw vector.
    pub fn to_bytes_large(&self) -> ZKResult<Vec<u8>> {
        user_to_bytes::<LargeLayout>(self)
    }
}

// ── User packet layout abstraction (DRY parse/serialize) ──────────────────

/// Private trait abstracting over small (28-byte) and large (72-byte)
/// user packet wire formats. Each layout specifies field sizes plus
/// how group_id and user_id are read/written at the byte level.
trait UserPacketLayout {
    const PACKET_SIZE: usize;
    const PASSWORD_LEN: usize;
    const NAME_LEN: usize;

    fn read_group_id(rdr: &mut Cursor<&[u8]>) -> ZKResult<String>;
    fn read_user_id(rdr: &mut Cursor<&[u8]>) -> ZKResult<String>;

    fn write_group_id(payload: &mut Vec<u8>, group_id: &str) -> ZKResult<()>;
    fn write_user_id(payload: &mut Vec<u8>, user_id: &str) -> ZKResult<()>;
}

struct SmallLayout;
struct LargeLayout;

impl UserPacketLayout for SmallLayout {
    const PACKET_SIZE: usize = USER_PACKET_SIZE_SMALL;
    const PASSWORD_LEN: usize = 5;
    const NAME_LEN: usize = 8;

    fn read_group_id(rdr: &mut Cursor<&[u8]>) -> ZKResult<String> {
        let _pad = rdr.read_u8()?;
        let group_id = rdr.read_u8()?;
        Ok(group_id.to_string())
    }

    fn read_user_id(rdr: &mut Cursor<&[u8]>) -> ZKResult<String> {
        let _timezone = rdr.read_u16::<LittleEndian>()?;
        let user_id = rdr.read_u32::<LittleEndian>()?;
        Ok(user_id.to_string())
    }

    fn write_group_id(payload: &mut Vec<u8>, group_id: &str) -> ZKResult<()> {
        payload.write_u8(0)?; // pad
        let id = group_id.parse::<u8>().map_err(|_| {
            ZKError::InvalidData(
                crate::ZKErrorCode::InvalidDataFormat,
                format!("Invalid group_id '{}': must be a u8 integer", group_id),
            )
        })?;
        payload.write_u8(id)?;
        Ok(())
    }

    fn write_user_id(payload: &mut Vec<u8>, user_id: &str) -> ZKResult<()> {
        payload.write_u16::<LittleEndian>(0)?; // timezone/pad
        let id = user_id.parse::<u32>().map_err(|_| {
            ZKError::InvalidData(
                crate::ZKErrorCode::InvalidDataFormat,
                format!("Invalid user_id '{}': must be a u32 integer", user_id),
            )
        })?;
        payload.write_u32::<LittleEndian>(id)?;
        Ok(())
    }
}

impl UserPacketLayout for LargeLayout {
    const PACKET_SIZE: usize = USER_PACKET_SIZE_LARGE;
    const PASSWORD_LEN: usize = 8;
    const NAME_LEN: usize = 24;

    fn read_group_id(rdr: &mut Cursor<&[u8]>) -> ZKResult<String> {
        let _pad1 = rdr.read_u8()?;
        let mut group_id_bytes = [0u8; 7];
        rdr.read_exact(&mut group_id_bytes)?;
        Ok(String::from_utf8_lossy(&group_id_bytes)
            .trim_matches('\0')
            .to_string())
    }

    fn read_user_id(rdr: &mut Cursor<&[u8]>) -> ZKResult<String> {
        let _pad2 = rdr.read_u8()?;
        let mut user_id_bytes = [0u8; 24];
        rdr.read_exact(&mut user_id_bytes)?;
        Ok(String::from_utf8_lossy(&user_id_bytes)
            .trim_matches('\0')
            .to_string())
    }

    fn write_group_id(payload: &mut Vec<u8>, group_id: &str) -> ZKResult<()> {
        payload.write_u8(0)?; // pad1
        let mut buf = [0u8; 7];
        let g_bytes = group_id.as_bytes();
        let len = std::cmp::min(g_bytes.len(), 7);
        buf[..len].copy_from_slice(&g_bytes[..len]);
        payload.write_all(&buf)?;
        Ok(())
    }

    fn write_user_id(payload: &mut Vec<u8>, user_id: &str) -> ZKResult<()> {
        payload.write_u8(0)?; // pad2
        let mut buf = [0u8; 24];
        let u_bytes = user_id.as_bytes();
        let len = std::cmp::min(u_bytes.len(), 24);
        buf[..len].copy_from_slice(&u_bytes[..len]);
        payload.write_all(&buf)?;
        Ok(())
    }
}

/// Generic parser: reads a `User` from a raw chunk using layout `L`.
fn parse_user<L: UserPacketLayout>(chunk: &[u8]) -> ZKResult<User> {
    if chunk.len() < L::PACKET_SIZE {
        return Err(ZKError::InvalidData(
            crate::ZKErrorCode::InvalidDataFormat,
            "User chunk too short".into(),
        ));
    }
    let mut rdr = Cursor::new(chunk);
    let uid = rdr.read_u16::<LittleEndian>()?;
    let privilege = rdr.read_u8()?;

    let mut password_bytes = vec![0u8; L::PASSWORD_LEN];
    rdr.read_exact(&mut password_bytes)?;
    let password = String::from_utf8_lossy(&password_bytes)
        .trim_matches('\0')
        .to_string();

    let mut name_bytes = vec![0u8; L::NAME_LEN];
    rdr.read_exact(&mut name_bytes)?;
    let name = crate::ZK::decode_gbk(&name_bytes);

    let card = rdr.read_u32::<LittleEndian>()?;
    let group_id = L::read_group_id(&mut rdr)?;
    let user_id = L::read_user_id(&mut rdr)?;

    Ok(User::new(
        uid, name, privilege, password, group_id, user_id, card,
    ))
}

/// Generic serializer: writes a `User` into a raw vector using layout `L`.
fn user_to_bytes<L: UserPacketLayout>(user: &User) -> ZKResult<Vec<u8>> {
    let mut payload = Vec::with_capacity(L::PACKET_SIZE);
    payload.write_u16::<LittleEndian>(user.uid)?;
    payload.write_u8(user.privilege)?;

    let mut password_bytes = vec![0u8; L::PASSWORD_LEN];
    let p_bytes = user.password.as_bytes();
    let p_len = std::cmp::min(p_bytes.len(), L::PASSWORD_LEN);
    password_bytes[..p_len].copy_from_slice(&p_bytes[..p_len]);
    payload.write_all(&password_bytes)?;

    let mut name_bytes = vec![0u8; L::NAME_LEN];
    let n_bytes_gbk = encoding_rs::GBK.encode(&user.name).0;
    let n_len = std::cmp::min(n_bytes_gbk.len(), L::NAME_LEN);
    name_bytes[..n_len].copy_from_slice(&n_bytes_gbk[..n_len]);
    payload.write_all(&name_bytes)?;

    payload.write_u32::<LittleEndian>(user.card)?;
    L::write_group_id(&mut payload, &user.group_id)?;
    L::write_user_id(&mut payload, &user.user_id)?;

    Ok(payload)
}

/// Represents a fingerprint template.
#[derive(Debug, Clone)]
pub struct Finger {
    /// UID of the user this finger belongs to.
    uid: u16,
    /// Finger ID (0-9).
    fid: u8,
    /// Whether the template is valid.
    valid: u8,
    /// The raw binary fingerprint template data.
    template: Vec<u8>,
}

impl Finger {
    /// Creates a new Finger record.
    pub fn new(uid: u16, fid: u8, valid: u8, template: Vec<u8>) -> Self {
        Self {
            uid,
            fid,
            valid,
            template,
        }
    }

    /// Getter for uid.
    pub fn uid(&self) -> u16 {
        self.uid
    }

    /// Getter for fid.
    pub fn fid(&self) -> u8 {
        self.fid
    }

    /// Getter for valid.
    pub fn valid(&self) -> u8 {
        self.valid
    }

    /// Getter for template.
    pub fn template(&self) -> &[u8] {
        &self.template
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_user_privileges() {
        let user = User {
            uid: 1,
            name: "Test".to_string(),
            privilege: 0, // Default enabled
            password: "".to_string(),
            group_id: "1".to_string(),
            user_id: "1".to_string(),
            card: 0,
        };
        assert!(user.is_enabled());
        assert!(!user.is_disabled());

        let disabled_user = User {
            privilege: 1, // Disabled bit set
            ..user.clone()
        };
        assert!(disabled_user.is_disabled());
        assert!(!disabled_user.is_enabled());

        let admin_user = User {
            privilege: 14, // USER_ADMIN
            ..user
        };
        assert_eq!(admin_user.user_type(), 14);
    }

    #[test]
    fn test_attendance_time_logic() {
        use chrono::NaiveDateTime;

        let naive =
            NaiveDateTime::parse_from_str("2026-02-19 09:16:41", "%Y-%m-%d %H:%M:%S").unwrap();

        // 1. Test UTC+7 (Vietnam)
        let att_vn = Attendance {
            uid: 1,
            user_id: "101".to_string(),
            timestamp: naive,
            status: 1,
            punch: 0,
            timezone_offset: 420, // 7 * 60
        };
        assert_eq!(att_vn.iso_format(), "2026-02-19T09:16:41+07:00");
        assert_eq!(
            att_vn.timestamp_utc().unwrap().to_rfc3339(),
            "2026-02-19T02:16:41+00:00"
        );

        // 2. Test UTC-5 (New York)
        let att_ny = Attendance {
            timezone_offset: -300, // -5 * 60
            ..att_vn.clone()
        };
        assert_eq!(att_ny.iso_format(), "2026-02-19T09:16:41-05:00");
        assert_eq!(
            att_ny.timestamp_utc().unwrap().to_rfc3339(),
            "2026-02-19T14:16:41+00:00"
        );

        // 3. Test UTC+0
        let att_utc = Attendance {
            timezone_offset: 0,
            ..att_vn.clone()
        };
        assert_eq!(att_utc.iso_format(), "2026-02-19T09:16:41+00:00");
    }

    #[test]
    fn test_attendance_safety_fallback() {
        use chrono::NaiveDateTime;
        let naive =
            NaiveDateTime::parse_from_str("2026-02-19 09:16:41", "%Y-%m-%d %H:%M:%S").unwrap();

        // Test with an invalid offset (e.g., 25 hours = 1500 minutes)
        // Our sanity check in timestamp_fixed should return None for offsets > 24h
        let att_invalid = Attendance {
            uid: 1,
            user_id: "101".to_string(),
            timestamp: naive,
            status: 1,
            punch: 0,
            timezone_offset: 1500,
        };

        // Should not panic, should return None
        assert!(att_invalid.timestamp_fixed().is_none());
        assert!(att_invalid.timestamp_utc().is_none());

        // ISO format should fallback to naive representation
        assert_eq!(att_invalid.iso_format(), "2026-02-19T09:16:41");
    }

    #[test]
    fn test_uncovered_models() {
        let naive = chrono::NaiveDate::from_ymd_opt(2024, 3, 9)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        let att = Attendance::new(10, "101".to_string(), naive, 1, 3, 420);
        assert_eq!(att.uid(), 10);
        assert_eq!(att.user_id(), "101");
        assert_eq!(att.timestamp(), naive);
        assert_eq!(att.status(), 1);
        assert_eq!(att.punch(), 3);
        assert!(att.timestamp_fixed().is_some());

        let mut user = User {
            uid: 1,
            name: "Initial".to_string(),
            privilege: 0,
            password: "pass".to_string(),
            group_id: "grp".to_string(),
            user_id: "1".to_string(),
            card: 0,
        };
        user.set_name("Updated".to_string());
        assert_eq!(user.name(), "Updated");
        assert_eq!(user.password(), "pass");
        assert_eq!(user.group_id(), "grp");
    }
}
