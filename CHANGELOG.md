# Changelog

All notable changes to this project will be documented in this file.

## [0.4.1] - 2026-02-20
### Added
- **User Management**: Implemented `set_user`, `delete_user`, and `refresh_data` for full administrative control over device users.
- **Conflict Protection**: Added automatic validation in `set_user` to prevent duplicate User IDs across different UIDs, ensuring data integrity.
- **Smart Indexing**: Added `get_next_free_uid` helper to automatically find available internal slots for new users.
- **Lookup Helpers**: Added `find_user_by_id` to easily retrieve user objects and their internal UIDs using employee IDs.
- **New User Tests**: Implemented comprehensive integration tests in `tests/user_tests.rs` covering add, update, conflict detection, and deletion flows.
- **New Management Example**: Added `examples/user_management.rs` demonstrating the complete lifecycle of user administration.

### Changed
- **Finalized Lazy Localization**: Refined the on-demand timezone synchronization logic to ensure it only runs once per session and only when time data is accessed.
- **Re-exported Constants**: Promoted protocol constants to the library root for easier access in user applications.
- **Safety Hardening**: Replaced potential panic points (`unwrap()`) in protocol serialization with robust `io::Result` error handling.
- **Documentation**: Added comprehensive doc comments to all public models (`User`, `Attendance`) and core library methods.
- **Example Refinement**: Standardized example usage and removed hardcoded fallback IPs for safer testing.

## [0.4.0] - 2026-02-20
### Added
- **Device Time Synchronization**: Implemented `set_time` and `encode_time` to allow updating the device's clock.
- **Lazy Timezone Detection**: Implemented on-demand timezone synchronization (`TZAdj`). The library automatically fetches and caches the device's offset the first time time-related data is accessed, ensuring localization without unnecessary network overhead during connection.
- **New Time Checker Example**: Added `examples/check_time.rs` to demonstrate standalone time retrieval and drift detection.
- **Test Suite Modernization**: Updated all mock servers to handle the new automated handshake and standardized on `from_bytes_owned` for high-performance packet parsing.
- **Zero-Copy Protocol Representation**: Refactored `ZKPacket` to use `std::borrow::Cow` for payloads, allowing zero-copy parsing from buffers.
- **Buffer-Centric Data Retrieval**: Introduced `read_chunk_into` and `receive_chunk_into` to allow streaming data directly into pre-allocated vectors, drastically reducing allocation overhead for large attendance logs.
- **Optimized TCP Wrapping**: Streamlined `send_command` to perform single-buffer allocation for wrapped TCP packets.
- **New Tests**: Added unit tests for time encoding, integration tests for `set_time`, and specialized `time_sync_tests` to verify lazy localization and offset caching.

### Changed
- Refined `read_packet` to eliminate redundant intermediate allocations during packet body reads.
- Updated internal data chunking loops to prioritize buffer reuse.

### Fixed
- Fixed a protocol misalignment in `read_packet` where TCP headers were incorrectly processed as part of the ZK body.

## [0.3.0] - 2026-02-19
### Added
- Standard `log` crate integration for better observability.
- Support for Hostnames in `connect_tcp` via DNS resolution.
- New regression tests for protocol misalignment and safety limits.

### Changed
- **Major Performance Optimization**: Refactored packet serialization and deserialization to use buffer-centric methods (`to_bytes_into`, `wrap_into`), significantly reducing memory allocations.
- **Zero-Copy Improvements**: Optimized `read_packet` to read directly into payload buffers and implemented `from_bytes_owned`.
- Consolidated transport reading logic into centralized internal helpers for better maintainability.

### Fixed
- Critical protocol desynchronization in `receive_chunk` by implementing strict `reply_id` validation.
- Infinite loop vulnerability by enforcing a `MAX_DISCARDED_PACKETS` limit on stale packets.
- Potential panic during TCP connection initialization.

## [0.2.9] - 2026-02-19
### Fixed
- Request-Response Misalignment in TCP/UDP by verifying `reply_id` in `send_command`.
- Improved TCP robustness using `read_exact` for header and payload separation.

## [0.2.6] - 2026-02-19
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

## [0.1.5] - 2026-02-18
### Added
- Credits and references to the original `fananimi/pyzk` library.

## [0.1.4] - 2026-02-18
### Fixed
- Fixed Crates.io publish workflow (deprecated flag and dirty state).

## [0.1.3] - 2026-02-18
### Fixed
- Fixed Clippy warnings and formatting issues to satisfy CI.

## [0.1.2] - 2026-02-18
### Added
- Comprehensive integration tests for authentication handshake.
- Extended unit tests for `make_commkey` with more edge cases.
- Formal test reports and focused QA verification.

## [0.1.1] - 2026-02-18
### Added
- Authentication support (`ZK::set_password`, `make_commkey`, `CMD_AUTH`).
- Unit tests for authentication logic.
- Documentation for password configuration.

## [0.1.0] - 2026-02-18
### Added
- Initial Rust port of `pyzk2`.
- Binary protocol implementation (UDP/TCP).
- Device info retrieval (SN, Firmware, MAC).
- Attendance log retrieval.
- User list retrieval.
- System controls (Restart, Poweroff, Unlock).
- Memory capacity tracking.
