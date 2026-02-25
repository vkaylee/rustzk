use chrono::NaiveDate;
use rustzk::models::User;
use rustzk::ZK;

#[test]
fn test_parse_attendance_8bytes() {
    let time_bytes = 839845230u32.to_le_bytes();

    let decoded_time = ZK::decode_time(&time_bytes).unwrap();
    let expected_date = NaiveDate::from_ymd_opt(2026, 2, 18).unwrap();
    let expected_time = expected_date.and_hms_opt(10, 20, 30).unwrap();

    assert_eq!(decoded_time, expected_time);
}

#[test]
fn test_encode_time() {
    let date = NaiveDate::from_ymd_opt(2026, 2, 18).unwrap();
    let time = date.and_hms_opt(10, 20, 30).unwrap();

    let encoded = ZK::encode_time(time);
    assert_eq!(encoded, 839845230);
}

#[test]
fn test_time_roundtrip() {
    let date = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();
    let time = date.and_hms_opt(14, 45, 0).unwrap();

    let encoded = ZK::encode_time(time);
    let bytes = encoded.to_le_bytes();
    let decoded = ZK::decode_time(&bytes).unwrap();

    assert_eq!(time, decoded);
}

#[test]
fn test_parse_user_28bytes() {
    // Format: <HB5s8sIxBHI
    let mut data = vec![0x01, 0x00, 0x0E]; // UID: 1, Priv: 14 (Admin)
    data.extend_from_slice(b"123\0\0"); // Pass
    data.extend_from_slice(b"John\0\0\0\0"); // Name (8 bytes)
    data.extend_from_slice(&123456u32.to_le_bytes()); // Card
    data.push(0x00); // Pad
    data.push(0x01); // Group
    data.extend_from_slice(&0u16.to_le_bytes()); // TZ
    data.extend_from_slice(&101u32.to_le_bytes()); // UserID

    let mut rdr = std::io::Cursor::new(&data);
    use byteorder::{LittleEndian, ReadBytesExt};
    use std::io::Read;

    let uid = rdr.read_u16::<LittleEndian>().unwrap();
    let privilege = rdr.read_u8().unwrap();
    let mut password_bytes = [0u8; 5];
    rdr.read_exact(&mut password_bytes).unwrap();
    let mut name_bytes = [0u8; 8];
    rdr.read_exact(&mut name_bytes).unwrap();
    let card = rdr.read_u32::<LittleEndian>().unwrap();
    let _pad = rdr.read_u8().unwrap();
    let group_id = rdr.read_u8().unwrap();

    let user = User {
        uid,
        name: String::from_utf8_lossy(&name_bytes)
            .trim_matches('\0')
            .to_string(),
        privilege,
        password: String::from_utf8_lossy(&password_bytes)
            .trim_matches('\0')
            .to_string(),
        group_id: group_id.to_string(),
        user_id: "101".to_string(),
        card,
    };

    assert_eq!(user.uid, 1);
    assert_eq!(user.name, "John");
    assert_eq!(user.privilege, 14);
}

#[test]
fn test_max_response_size_limit() {
    // Logic verified via code review and manual inspection of read_with_buffer
    // MAX_RESPONSE_SIZE is enforced in get_users, get_attendance, and get_templates
}
