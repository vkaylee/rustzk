use chrono::NaiveDateTime;

#[derive(Debug, Clone)]
pub struct Attendance {
    pub uid: u32,
    pub user_id: String,
    pub timestamp: NaiveDateTime,
    pub status: u8,
    pub punch: u8,
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
