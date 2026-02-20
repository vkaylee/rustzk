use chrono::{DateTime, FixedOffset, NaiveDateTime, TimeZone, Utc};

/// Represents an attendance record (clock-in/out).
#[derive(Debug, Clone)]
pub struct Attendance {
    /// Internal record UID (sequence number).
    pub uid: u32,
    /// The user ID string associated with the record.
    pub user_id: String,
    /// The raw timestamp from the device.
    pub timestamp: NaiveDateTime,
    /// Attendance status code.
    pub status: u8,
    /// Punch type (e.g., finger, face, card).
    pub punch: u8,
    /// The timezone offset in minutes applied to this record.
    pub timezone_offset: i32,
}

impl Attendance {
    /// Returns the timestamp as a DateTime with the device's fixed offset.
    pub fn timestamp_fixed(&self) -> DateTime<FixedOffset> {
        let offset = FixedOffset::east_opt(self.timezone_offset * 60)
            .unwrap_or_else(|| FixedOffset::east_opt(0).unwrap());
        offset
            .from_local_datetime(&self.timestamp)
            .single()
            .unwrap_or_else(|| {
                // Fallback for ambiguous or non-existent times during DST transitions
                DateTime::<FixedOffset>::from_naive_utc_and_offset(self.timestamp, offset)
            })
    }

    /// Returns the timestamp in UTC.
    pub fn timestamp_utc(&self) -> DateTime<Utc> {
        let fixed = self.timestamp_fixed();
        fixed.with_timezone(&Utc)
    }

    /// Returns the timestamp formatted as an ISO8601 string with offset.
    pub fn iso_format(&self) -> String {
        self.timestamp_fixed().to_rfc3339()
    }
}

/// Represents a user on the ZK device.
#[derive(Debug, Clone)]
pub struct User {
    /// Internal user UID.
    pub uid: u16,
    /// User's display name.
    pub name: String,
    /// User's privilege level (Admin, User, etc.).
    pub privilege: u8,
    /// User's numeric password (if any).
    pub password: String,
    /// ID of the group the user belongs to.
    pub group_id: String,
    /// The alphanumeric user ID string.
    pub user_id: String,
    /// ID of the proximity card assigned to the user.
    pub card: u32,
}

impl User {
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
}

/// Represents a fingerprint template.
#[derive(Debug, Clone)]
pub struct Finger {
    /// UID of the user this finger belongs to.
    pub uid: u16,
    /// Finger ID (0-9).
    pub fid: u8,
    /// Whether the template is valid.
    pub valid: u8,
    /// The raw binary fingerprint template data.
    pub template: Vec<u8>,
}

#[cfg(test)]
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
            att_vn.timestamp_utc().to_rfc3339(),
            "2026-02-19T02:16:41+00:00"
        );

        // 2. Test UTC-5 (New York)
        let att_ny = Attendance {
            timezone_offset: -300, // -5 * 60
            ..att_vn.clone()
        };
        assert_eq!(att_ny.iso_format(), "2026-02-19T09:16:41-05:00");
        assert_eq!(
            att_ny.timestamp_utc().to_rfc3339(),
            "2026-02-19T14:16:41+00:00"
        );

        // 3. Test UTC+0
        let att_utc = Attendance {
            timezone_offset: 0,
            ..att_vn.clone()
        };
        assert_eq!(att_utc.iso_format(), "2026-02-19T09:16:41+00:00");
    }
}
