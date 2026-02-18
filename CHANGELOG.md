# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]
### Added
- Stateful Mock ZK Server for integration testing.
- Comprehensive integration tests for `get_users` and `get_attendance`.
- Data parsing tests for 8-byte and 28-byte ZK formats.
- Memory safety limits (`MAX_RESPONSE_SIZE`) to prevent DoS via large allocations.
- GitHub Actions CI workflow for automated testing and linting.
- Automated crates.io publishing workflow on version tag push.

### Fixed
- TCP header decoding logic to handle 8-byte magic numbers separately from payload.
- Byte alignment and padding issues in User record parsing.
- Improved authentication error reporting when device returns `CMD_ACK_UNAUTH`.

## [0.1.0] - 2026-02-18
### Added
- Initial Rust port of `pyzk2`.
- Binary protocol implementation (UDP/TCP).
- Device info retrieval (SN, Firmware, MAC).
- Attendance log retrieval.
- User list retrieval.
- System controls (Restart, Poweroff, Unlock).
- Memory capacity tracking.
