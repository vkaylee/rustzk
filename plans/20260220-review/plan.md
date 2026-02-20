# Review Implementation Plan - 20260220

## Overview
This plan addresses the issues identified during the code review of the `rustzk` codebase, focusing on safety, protocol fidelity, and robustness.

## Phase 1: Safety & Protocol Fidelity (High/Medium)
1. **Fix `Drop` implementation in `src/lib.rs`**:
   - Remove the blocking `CMD_EXIT` network call from `Drop`.
   - Ensure `Drop` only cleans up local resources (resets `transport` and `is_connected`).
   - Recommend users call `disconnect()` explicitly for a clean protocol-level exit.
2. **Align Checksum Logic in `src/protocol.rs`**:
   - Verify the 1-off discrepancy against `pyzk_ref`.
   - Implement the exact one's complement logic: `~sum + USHRT_MAX` if negative, to ensure compatibility with all firmware versions.
   - Add unit tests for boundary cases (e.g., sum = 0).

## Phase 2: Robustness & Maintainability (Medium/Low)
1. **Robust User Parsing**:
   - Refactor `get_users` to handle unknown packet sizes more gracefully instead of silent failures or misaligned reads.
   - Add logging for unknown packet sizes.
2. **GBK Decoding Safety**:
   - Update `decode_gbk` to log a warning if replacement characters are found, indicating potential encoding issues.
3. **Refactor Magic Numbers**:
   - Move hardcoded offsets in `get_attendance` and `get_users` to constants in `constants.rs`.

## Verification Strategy
- **Unit Tests**: `cargo test` to ensure checksum and parsing logic remain correct.
- **Integration Tests**: Run `tests/drop_tests.rs` and `tests/parsing_tests.rs`.
- **Manual Verification**: Run `examples/check_device_info.rs` (mocked) to verify connection/disconnection flow.
