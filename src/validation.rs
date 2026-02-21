//! Input validation utilities for rustzk
//!
//! This module provides comprehensive validation for all network inputs
//! to ensure security and prevent malformed data attacks.

use crate::{ZKError, ZKResult};

/// Validates protocol header fields
///
/// # Arguments
/// * `header` - Raw header bytes
///
/// # Returns
/// * `Ok(())` if header is valid
/// * `Err(ZKError::InvalidData)` if header is malformed
pub fn validate_protocol_header(header: &[u8]) -> ZKResult<()> {
    // Ensure minimum header size
    if header.len() < 8 {
        return Err(ZKError::InvalidData(
            "Header too short: minimum 8 bytes required".into(),
        ));
    }

    // Validate packet length from header
    let length = u16::from_le_bytes([header[0], header[1]]) as usize;

    // Check for reasonable minimum size
    if length < 4 {
        return Err(ZKError::InvalidData(
            "Packet length too small: minimum 4 bytes required".into(),
        ));
    }

    // Apply security bounds
    crate::security::validate_packet_size(length)?;

    // Validate protocol version if present
    if header.len() > 8 {
        let version = header[8];
        if version != 0x01 && version != 0x02 {
            return Err(ZKError::InvalidData(format!(
                "Unsupported protocol version: 0x{:02x}",
                version
            )));
        }
    }

    Ok(())
}

/// Validates command codes
///
/// # Arguments
/// * `command` - Command byte to validate
///
/// # Returns
/// * `Ok(())` if command is recognized
/// * `Err(ZKError::InvalidData)` if command is unknown
pub fn validate_command(command: u8) -> ZKResult<()> {
    const VALID_COMMANDS: &[u8] = &[
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x50, 0x51, 0x52, 0x53, 0x54,
        0x55, 0x56, 0x57, 0x58, 0x59, 0x60, 0x61, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,
        0x80, 0x81, 0x82, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89, 0x8a, 0x8b, 0x8c, 0x8d, 0x8e,
        0x8f, 0x90, 0x91, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9a, 0x9b, 0x9c, 0x9d,
        0x9e, 0x9f,
    ];

    if !VALID_COMMANDS.contains(&command) {
        return Err(ZKError::InvalidData(format!(
            "Unknown command code: 0x{:02x}",
            command
        )));
    }

    Ok(())
}

/// Validates data payload structure
///
/// # Arguments
/// * `data` - Raw data bytes
/// * `expected_min_size` - Minimum expected size (0 for flexible)
///
/// # Returns
/// * `Ok(())` if data structure is valid
/// * `Err(ZKError::InvalidData)` if data is malformed
pub fn validate_data_payload(data: &[u8], expected_min_size: usize) -> ZKResult<()> {
    // Check minimum size requirement
    if data.len() < expected_min_size {
        return Err(ZKError::InvalidData(format!(
            "Data payload too small: expected at least {} bytes, got {}",
            expected_min_size,
            data.len()
        )));
    }

    // Validate size bounds
    crate::security::validate_packet_size(data.len())?;

    // Check for null byte injection in string fields
    if data.windows(2).any(|window| window == [0x00, 0x00]) {
        return Err(ZKError::InvalidData(
            "Invalid null byte sequence in data payload".into(),
        ));
    }

    Ok(())
}

/// Validates device ID format
///
/// # Arguments
/// * `device_id` - Device ID string bytes
///
/// # Returns
/// * `Ok(())` if device ID is valid
/// * `Err(ZKError::InvalidData)` if device ID is malformed
pub fn validate_device_id(device_id: &[u8]) -> ZKResult<()> {
    // Device IDs should be reasonable length (typically < 32 bytes)
    if device_id.len() > 32 {
        return Err(ZKError::InvalidData(
            "Device ID too long: maximum 32 bytes".into(),
        ));
    }

    // Device IDs should be printable ASCII or valid UTF-8
    if !device_id.iter().all(|&b| b.is_ascii_graphic() || b == 0) {
        return Err(ZKError::InvalidData(
            "Device ID contains invalid characters".into(),
        ));
    }

    Ok(())
}

/// Comprehensive validation for network packets
///
/// # Arguments
/// * `header` - Packet header bytes
/// * `data` - Packet data bytes
/// * `command` - Command code
///
/// # Returns
/// * `Ok(())` if packet is fully valid
/// * `Err(ZKError::InvalidData)` with specific error details
pub fn validate_network_packet(header: &[u8], data: &[u8], command: u8) -> ZKResult<()> {
    // Validate all components
    validate_protocol_header(header)?;
    validate_command(command)?;
    validate_data_payload(data, 0)?;

    // Additional cross-field validation
    let header_length = u16::from_le_bytes([header[0], header[1]]) as usize;
    if header_length != data.len() {
        return Err(ZKError::InvalidData(format!(
            "Header length ({}) doesn't match data length ({})",
            header_length,
            data.len()
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_protocol_header() {
        let header = [0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        assert!(validate_protocol_header(&header).is_ok());
    }

    #[test]
    fn test_invalid_protocol_header_too_short() {
        let header = [0x08, 0x00];
        assert!(validate_protocol_header(&header).is_err());
    }

    #[test]
    fn test_invalid_protocol_header_too_large() {
        std::env::set_var("RUSTZK_MAX_PACKET_SIZE", "32768"); // Set to 32KB for this test
        let mut header = [0u8; 8];
        header[0] = 0xFF;
        header[1] = 0xFF; // 65535 bytes, exceeds our 32KB test limit
        assert!(validate_protocol_header(&header).is_err());
        std::env::remove_var("RUSTZK_MAX_PACKET_SIZE");
    }

    #[test]
    fn test_valid_command() {
        assert!(validate_command(0x01).is_ok());
        assert!(validate_command(0x50).is_ok());
        assert!(validate_command(0x9f).is_ok());
    }

    #[test]
    fn test_invalid_command() {
        assert!(validate_command(0xFF).is_err());
        assert!(validate_command(0x10).is_err());
    }

    #[test]
    fn test_data_payload_validation() {
        let data = vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A]; // Non-null data
        assert!(validate_data_payload(&data, 5).is_ok());
        assert!(validate_data_payload(&data, 15).is_err());

        // Test null byte injection detection
        let null_data = vec![0x01, 0x00, 0x00, 0x04];
        assert!(validate_data_payload(&null_data, 0).is_err());
    }

    #[test]
    fn test_device_id_validation() {
        let valid_id = b"Device123";
        assert!(validate_device_id(valid_id).is_ok());

        let invalid_id = vec![0xFF; 50];
        assert!(validate_device_id(&invalid_id).is_err());
    }

    #[test]
    fn test_comprehensive_packet_validation() {
        let header = [0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let data = vec![0x01, 0x02, 0x03, 0x04];
        let command = 0x01;

        assert!(validate_network_packet(&header, &data, command).is_ok());
    }

    #[test]
    fn test_packet_length_mismatch() {
        let header = [0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let data = vec![0x01, 0x02, 0x03, 0x04]; // Only 4 bytes, header says 8
        let command = 0x01;

        assert!(validate_network_packet(&header, &data, command).is_err());
    }
}
