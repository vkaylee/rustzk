# Implementation Plan: Test Suite for RustZK

### ## Approach
- **Unit Testing**: Focus on core protocol logic (checksum, packet serialization) and model helpers. Since these are pure functions or deterministic state changes, they are ideal for standard `#[test]` modules.
- **Mocking Strategy**: Since we cannot rely on a physical ZK device during CI/CD, we will implement a mock server or use trait-based injection to simulate device responses for higher-level functions like `get_attendance`.
- **Reference Validation**: Compare Rust output with known outputs from the `pyzk2` reference library for specific byte sequences.

### ## Steps
1. **Model Unit Tests** (10 min)
   - Add tests to `src/models.rs` for `User` methods (`is_enabled`, `user_type`).
   - Validate `NaiveDateTime` decoding in `Attendance`.

2. **Protocol Unit Tests** (15 min)
   - Files to modify: `src/protocol.rs`
   - Test cases:
     - `calculate_checksum` with known valid ZK packets.
     - `ZKPacket` serialization/deserialization.
     - `TCPWrapper` header wrapping/unwrapping.

3. **Client Connection & Command Tests** (20 min)
   - Files to create: `tests/client_tests.rs`
   - Implement a simple UDP/TCP mock listener to verify that `ZK::connect` and `ZK::send_command` send correctly formatted packets.

### ## Timeline
| Phase | Duration |
|-------|----------|
| Model Tests | 10 min |
| Protocol Tests | 15 min |
| Client Mocking | 20 min |
| **Total** | **45 min** |

### ## Rollback Plan
1. Delete `tests/` directory.
2. Remove `#[cfg(test)]` blocks from `src/`.

### ## Security Checklist
- [x] Check for integer overflows in checksum calculation.
- [x] Validate packet length bounds during `from_bytes` (Prevent OOM/Panic).
- [ ] Fuzzing for malicious device responses (Future scope).
