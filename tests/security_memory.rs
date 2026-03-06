//! Security tests for memory safety in rustzk
//!
//! This module comprehensively tests memory allocation boundaries
//! and ensures the prevention of memory exhaustion attacks.

use rustzk::{security, validation};
use std::sync::Mutex;
use std::sync::OnceLock;

/// Shared lock for environment variable modifications in tests
fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

/// Test that bounded memory allocation prevents oversize packets
#[test]
fn test_bounded_memory_allocation() {
    // Test normal sized packet (should work)
    assert!(security::validate_packet_size(1024).is_ok());
    assert!(security::validate_packet_size(security::MAX_PACKET_SIZE).is_ok());

    // Test oversized packet (should fail)
    let oversized = security::MAX_PACKET_SIZE + 1;
    assert!(security::validate_packet_size(oversized).is_err());
}

/// Test header validation prevents malformed headers
#[test]
fn test_protocol_header_validation() {
    // Valid header
    let valid_header = [0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    assert!(validation::validate_protocol_header(&valid_header).is_ok());

    // Header too short
    let short_header = [0x08, 0x00];
    assert!(validation::validate_protocol_header(&short_header).is_err());

    // Header claiming excessive size - test with environment override
    let _lock = env_lock().lock().unwrap();
    std::env::set_var("RUSTZK_MAX_PACKET_SIZE", "32768"); // Set to 32KB for this test
    let mut oversized_header = [0u8; 8];
    oversized_header[0] = 0xFF; // 65535 bytes, exceeds our 32KB test limit
    oversized_header[1] = 0xFF;
    assert!(validation::validate_protocol_header(&oversized_header).is_err());
    std::env::remove_var("RUSTZK_MAX_PACKET_SIZE"); // Clean up
}

/// Test network packet validation components
#[test]
fn test_network_packet_validation() {
    let header = [0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    let data = vec![0x01, 0x02, 0x03, 0x04];
    let command = 0x01;

    // Valid packet should pass
    assert!(validation::validate_network_packet(&header, &data, command).is_ok());

    // Mismatched lengths should fail
    let wrong_header = [0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    assert!(validation::validate_network_packet(&wrong_header, &data, command).is_err());

    // Invalid command should fail
    let bad_command = 0xFF;
    assert!(validation::validate_network_packet(&header, &data, bad_command).is_err());
}

/// Test memory exhaustion protection
#[test]
fn test_memory_exhaustion_protection() {
    let _lock = env_lock().lock().unwrap();
    // Test environment variable override
    std::env::set_var("RUSTZK_MAX_PACKET_SIZE", "2097152"); // 2MB

    // Should accept larger limit
    assert!(security::validate_packet_size(1_500_000).is_ok());

    // Still should enforce the new limit
    assert!(security::validate_packet_size(2_500_000).is_err());

    // Clean up
    std::env::remove_var("RUSTZK_MAX_PACKET_SIZE");
}

/// Test data payload validation
#[test]
fn test_data_payload_validation() {
    let good_data = vec![0x01, 0x02, 0x03, 0x04, 0x05];
    assert!(validation::validate_data_payload(&good_data, 3).is_ok());
    assert!(validation::validate_data_payload(&good_data, 10).is_err());

    // Test null byte injection detection
    let injection_data = vec![0x01, 0x00, 0x00, 0x04];
    assert!(validation::validate_data_payload(&injection_data, 0).is_err());
}

/// Test device ID validation
#[test]
fn test_device_id_validation() {
    let valid_id = b"Device123";
    assert!(validation::validate_device_id(valid_id).is_ok());

    let long_id = vec![b'X'; 50]; // Too long
    assert!(validation::validate_device_id(&long_id).is_err());

    let invalid_chars = vec![0xFF, 0xFE, 0xFD];
    assert!(validation::validate_device_id(&invalid_chars).is_err());
}

/// Rapid allocation test to prevent DoS
#[test]
fn test_rapid_allocation_boundaries() {
    let iterations = 1000;
    let test_size = 8192; // 8KB per allocation

    for i in 0..iterations {
        // Create varying sizes but all within bounds
        let size = test_size + (i % 1024);
        assert!(size <= security::MAX_PACKET_SIZE);

        // Simulate allocation attempts
        if security::validate_packet_size(size).is_ok() {
            let _test_data = vec![0u8; size];
            // Should succeed and not crash
        }
    }
}

/// Test boundary conditions around limits
#[test]
fn test_boundary_conditions() {
    let max_size = security::get_max_packet_size();

    // Exactly at limit should pass
    assert!(security::validate_packet_size(max_size).is_ok());

    // One byte over limit should fail
    assert!(security::validate_packet_size(max_size + 1).is_err());

    // Zero size should pass (if that's valid in protocol)
    assert!(validation::validate_data_payload(&[], 0).is_ok());
}

#[cfg(test)]
mod stress_tests {
    use super::*;
    use std::time::Instant;

    /// Stress test with many near-boundary allocations
    #[test]
    fn test_stress_near_boundary_conditions() {
        let start = Instant::now();
        // Use compile-time constant to avoid env var race with other concurrent tests
        let max_size = security::MAX_PACKET_SIZE;
        let test_iterations = 100;

        for _i in 0..test_iterations {
            // Test sizes at 90%, 95%, 99% of max
            let sizes = [
                (max_size as f64 * 0.9) as usize,
                (max_size as f64 * 0.95) as usize,
                (max_size as f64 * 0.99) as usize,
            ];

            for size in &sizes {
                assert!(
                    *size <= max_size,
                    "Size {} exceeds MAX_PACKET_SIZE {}",
                    size,
                    max_size
                );

                // Validate that we could actually allocate this
                let test_data = vec![0u8; *size];
                assert_eq!(test_data.len(), *size);
            }
        }

        let duration = start.elapsed();
        println!(
            "Stress test completed: {:?} for {} iterations",
            duration, test_iterations
        );

        // Ensure performance is reasonable
        assert!(
            duration.as_millis() < 5000,
            "Stress test too slow: {:?}",
            duration
        );
    }
}
