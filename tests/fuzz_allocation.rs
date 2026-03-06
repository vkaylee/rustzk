//! Fuzzing tests for memory allocation safety
//!
//! These tests systematically explore memory allocation boundary conditions
//! to prevent memory exhaustion and buffer overflow vulnerabilities.

use rustzk::{security, validation};
use std::sync::Mutex;
use std::sync::OnceLock;

/// Shared lock for environment variable modifications in tests
fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

/// Fuzz test TCP header parsing with malformed data
#[test]
fn fuzz_tcp_header_parsing() {
    let test_cases = [
        // Edge cases
        vec![0x00, 0x00],                                     // Too short
        vec![0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF], // Max values
        vec![0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], // Valid but minimal
        vec![0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], // Large first field
        // Random-like patterns
        vec![0xA5, 0x5A, 0x3C, 0xC3, 0x96, 0x69, 0x0F, 0xF0],
        vec![0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0],
        // Zero-filled
        vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
        // Pattern with potential integer overflow
        vec![0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00],
    ];

    for (i, test_data) in test_cases.iter().enumerate() {
        println!("Testing case {}: {:?}", i, test_data);

        // Should handle gracefully without panic
        let result = validation::validate_protocol_header(test_data);

        // Some should fail, some might pass, but none should crash
        match result {
            Ok(_) => println!("  Case {} accepted", i),
            Err(_) => println!("  Case {} rejected (expected)", i),
        }
    }
}

/// Fuzz test with varying packet sizes including boundary values
#[test]
fn fuzz_packet_size_validation() {
    let _lock = env_lock().lock().unwrap();

    let test_sizes = vec![
        0, 1, 2, 3, 4, // Minimum sizes
        7, 8, 15, 16, // Power-of-2 boundaries
        1023, 1024, 1025, // 1KB boundary
        65535, 65536, 65537, // 64KB boundary
        1048575, 1048576, 1048577, // 1MB boundary (default max)
        2097151, 2097152, 2097153, // 2MB boundary
        16777215, 16777216, 16777217, // 16MB boundary (legacy max)
        33554431, 33554432, 67108863, 134217727, // Larger values
    ];

    for size in test_sizes.iter() {
        let result = security::validate_packet_size(*size);
        let current_max = security::get_max_packet_size();

        if *size <= current_max {
            assert!(
                result.is_ok(),
                "Size {} should be valid (current max: {})",
                size,
                current_max
            );
        } else {
            assert!(
                result.is_err(),
                "Size {} should be invalid (current max: {})",
                size,
                current_max
            );
        }
    }
}

/// Fuzz test with rapid allocation and deallocation pattern
#[test]
fn fuzz_rapid_allocation_pattern() {
    let patterns = vec![
        // Small to large allocation pattern
        vec![64, 128, 256, 512, 1024, 2048, 4096, 8192],
        // Large to small allocation pattern
        vec![8192, 4096, 2048, 1024, 512, 256, 128, 64],
        // Alternating pattern
        vec![64, 8192, 128, 4096, 256, 2048, 512, 1024],
        // Spike pattern
        vec![64, 64, 64, 8192, 64, 64, 8192, 64],
        // Random-like pattern
        vec![123, 456, 789, 321, 654, 987, 147, 258],
    ];

    for pattern in &patterns {
        for size in pattern {
            assert!(
                security::validate_packet_size(*size).is_ok(),
                "Pattern size {} should be valid",
                size
            );

            // Simulate the actual allocation
            let test_data = vec![0u8; *size];
            assert_eq!(test_data.len(), *size);
        }
    }
}

/// Fuzz test malformed UDP packet handling
#[test]
fn fuzz_udp_packet_handling() {
    let test_packets = vec![
        // Empty packet
        vec![],
        // Single byte packets
        vec![0x00],
        vec![0xFF],
        vec![0x50],
        // Oversized packets
        vec![0xFF; 2049],                          // Over UDP buffer size
        vec![0xFF; security::MAX_PACKET_SIZE + 1], // Over security limit
        // Boundary sized packets
        vec![0x41; 2048], // Exactly UDP buffer size
        vec![0x42; 1024], // Half UDP buffer size
        // Packets with null bytes
        vec![0x00, 0x00, 0x00, 0x00],
        vec![0x01, 0x00, 0x00, 0x01],
        // Malformed headers
        vec![0xFF, 0xFE, 0xFD, 0xFC, 0xFB, 0xFA, 0xF9, 0xF8],
        vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08],
    ];

    for (i, packet) in test_packets.iter().enumerate() {
        println!("Testing UDP packet {} (size: {})", i, packet.len());

        // Validate packet size (should handle gracefully)
        let size_result = security::validate_packet_size(packet.len());

        // Validate data payload (should handle gracefully)
        let data_result = validation::validate_data_payload(packet, 0);

        // Should never panic - either accept or reject gracefully
        match (size_result, data_result) {
            (Ok(_), Ok(_)) => println!("  UDP packet {} accepted", i),
            _ => println!("  UDP packet {} rejected (may be expected)", i),
        }
    }
}

/// Fuzz test with environment variable changes
#[test]
fn fuzz_environment_variable_changes() {
    let _lock = env_lock().lock().unwrap();

    let test_limits = vec![
        "1024", "8192", "65536", "262144", "1048576", "2097152", "16777216",
    ];

    for limit_str in &test_limits {
        // Set environment variable
        std::env::set_var("RUSTZK_MAX_PACKET_SIZE", limit_str);

        // Check that the limit is respected
        let expected_limit: usize = limit_str.parse().unwrap();
        assert_eq!(security::get_max_packet_size(), expected_limit);

        // Test boundary around the new limit
        assert!(security::validate_packet_size(expected_limit).is_ok());
        assert!(security::validate_packet_size(expected_limit - 1).is_ok());
        assert!(security::validate_packet_size(expected_limit + 1).is_err());
    }

    // Clean up
    std::env::remove_var("RUSTZK_MAX_PACKET_SIZE");
}

/// Stress test with concurrent-like allocation patterns
#[test]
fn stress_concurrent_allocation_pattern() {
    let mut handles = Vec::new();
    let test_iterations = 10;

    for thread_id in 0..test_iterations {
        let handle = std::thread::spawn(move || {
            let base_size = 1024; // 1KB base
            let variance = 512; // ±512 bytes variance

            for i in 0..100 {
                let size = base_size + ((i + thread_id) % variance);

                // Use compile-time constant to avoid env var race with other tests
                assert!(
                    size <= security::MAX_PACKET_SIZE,
                    "Size {} exceeds MAX_PACKET_SIZE {}",
                    size,
                    security::MAX_PACKET_SIZE
                );
                let _test_data = vec![thread_id as u8; size];
            }

            thread_id
        });

        handles.push(handle);
    }

    // Wait for all threads
    for handle in handles {
        let result = handle.join().unwrap();
        println!("Thread {} completed successfully", result);
    }
}

/// Memory exhaustion simulation test
#[test]
fn test_memory_exhaustion_simulation() {
    // This test simulates an attacker trying to exhaust memory
    let mut total_allocated = 0;
    let max_safe_total = 50_000_000; // 50MB total limit for test
    let packet_size = 1024; // 1KB packets
    let safe_packet_count = max_safe_total / packet_size;

    for _i in 0..safe_packet_count {
        // Each allocation should be allowed
        assert!(security::validate_packet_size(packet_size).is_ok());

        // Simulate allocation
        let test_data = vec![0u8; packet_size];
        total_allocated += test_data.len();

        // Verify we're not exceeding safe bounds
        assert!(
            total_allocated <= max_safe_total,
            "Total allocated {} exceeds safe limit {}",
            total_allocated,
            max_safe_total
        );
    }

    println!(
        "Safely allocated {} bytes in {} packets",
        total_allocated, safe_packet_count
    );
}

/// Fuzz test with corrupted header patterns
#[test]
fn fuzz_corrupted_header_patterns() {
    let seed_headers = vec![
        vec![0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], // Valid header
        vec![0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], // Small valid header
        vec![0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00], // Large numbers
    ];

    for seed in &seed_headers {
        for corruption_byte in 0..=255 {
            let mut corrupted = seed.clone();

            // Corrupt each position with each possible byte value
            for pos in 0..corrupted.len() {
                corrupted[pos] = corruption_byte;

                // Should handle corruption gracefully
                let result = validation::validate_protocol_header(&corrupted);

                // None should cause panic
                let _ = result;
            }
        }
    }
}
