use chrono::{DateTime, FixedOffset, NaiveDateTime, TimeZone, Utc};

#[derive(Debug, Clone)]
pub struct Attendance {
    pub uid: u32,
    pub user_id: String,
    pub timestamp: NaiveDateTime,
    pub status: u8,
    pub punch: u8,
    pub timezone_offset: i32, // Offset in minutes (e.g., 420 for UTC+7)
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

#[derive(Debug, Clone)]
pub struct User {
    pub uid: u16,
    pub name: String,
    pub privilege: u8,
    pub password: String,
    pub group_id: String,
    pub user_id: String,
    pub card: u32,
}

impl User {
    pub fn is_disabled(&self) -> bool {
        (self.privilege & 1) != 0
    }

    pub fn is_enabled(&self) -> bool {
        !self.is_disabled()
    }

    pub fn user_type(&self) -> u8 {
        self.privilege & 0xE
    }
}

#[derive(Debug, Clone)]
pub struct Finger {
    pub uid: u16,
    pub fid: u8,
    pub valid: u8,
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
}
