use chrono::{DateTime, FixedOffset, NaiveDateTime, TimeZone, Utc};

/// Represents an attendance record (clock-in/out).
#[derive(Debug, Clone)]
pub struct Attendance {
    /// Internal record UID (sequence number).
    pub(crate) uid: u32,
    /// The user ID string associated with the record.
    pub(crate) user_id: String,
    /// The raw timestamp from the device.
    pub(crate) timestamp: NaiveDateTime,
    /// Attendance status code.
    pub(crate) status: u8,
    /// Punch type (e.g., finger, face, card).
    pub(crate) punch: u8,
    /// The timezone offset in minutes applied to this record.
    pub(crate) timezone_offset: i32,
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
        let offset = FixedOffset::east_opt(self.timezone_offset * 60).unwrap_or_else(|| {
            #[allow(clippy::unwrap_used)]
            FixedOffset::east_opt(0).unwrap()
        });
        offset.from_local_datetime(&self.timestamp).single()
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
    pub(crate) uid: u16,
    /// User's display name.
    pub(crate) name: String,
    /// User's privilege level (Admin, User, etc.).
    pub(crate) privilege: u8,
    /// User's numeric password (if any).
    pub(crate) password: String,
    /// ID of the group the user belongs to.
    pub(crate) group_id: String,
    /// The alphanumeric user ID string.
    pub(crate) user_id: String,
    /// ID of the proximity card assigned to the user.
    pub(crate) card: u32,
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
}

/// Represents a fingerprint template.
#[derive(Debug, Clone)]
pub struct Finger {
    /// UID of the user this finger belongs to.
    pub(crate) uid: u16,
    /// Finger ID (0-9).
    pub(crate) fid: u8,
    /// Whether the template is valid.
    pub(crate) valid: u8,
    /// The raw binary fingerprint template data.
    pub(crate) template: Vec<u8>,
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
