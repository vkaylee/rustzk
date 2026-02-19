# Solution: Protocol Synchronization and Robustness Fixes

## Problem Statement
The `rustzk` library exhibited protocol desynchronization issues when communicating with ZK devices, particularly those sending data in chunks (e.g., large attendance logs). 
1. **Misalignment**: `receive_chunk` did not verify `reply_id`, causing it to process stale packets as valid data.
2. **TCP Over-reading**: The use of `stream.read` instead of `read_exact` caused the buffer to consume bytes from subsequent packets, breaking the protocol stream.
3. **Potential Infinite Loops**: The lack of a limit on discarded packets allowed a malicious or malfunctioning device to hang the client by sending a stream of invalid `reply_id` packets.
4. **Panic Risk**: `connect_tcp` used `unwrap()` on address parsing, causing crashes with hostnames.

## Solution Description
A series of targeted fixes and refactorings were applied to address these issues:

1.  **Strict `reply_id` Validation**: Implemented mandatory checks in both `send_command` and `receive_chunk` (consolidated into `read_packet`) to ensure the response matches the request.
2.  **Robust Packet Reading**:
    *   Refactored TCP reading to use `read_exact` for both the header (8 bytes) and the variable-length body.
    *   Consolidated TCP and UDP packet reading logic into a single private helper method `read_packet` to eliminate code duplication.
3.  **Safety Limits**: Introduced `MAX_DISCARDED_PACKETS` (100) constant. The `read_packet` loop now errors out if it discards more than this limit, preventing infinite loops.
4.  **Connection Safety**: Updated `connect_tcp` to use `std::net::ToSocketAddrs`, enabling safe and robust DNS resolution for hostnames.
5.  **Logging**: Replaced ad-hoc `eprintln!` with the `log` crate for standard, configurable logging.

## Verification
-   **Unit/Regression Tests**: 
    -   `test_receive_chunk_misalignment_tcp`: Confirmed fix for stale packet injection.
    -   `test_max_discarded_packets_limit`: Confirmed safety limit triggers error.
    -   All 31 existing tests passed.
-   **Live Device Check**:
    -   Device: ZAM180_TFT (Ver 6.60) at `192.168.12.13`.
    -   Operation: `get_attendance` (21,793 records).
    -   Result: Success. Logs showed ~18 mismatched packets were correctly caught and discarded without data corruption.

## Key Files Changed
-   `src/lib.rs`: Core logic refactoring (`read_packet`, `connect_tcp`).
-   `src/constants.rs`: Added `MAX_DISCARDED_PACKETS`.
-   `tests/misalignment_tests.rs`: Added regression test.
-   `tests/safety_tests.rs`: Added safety test.

## Future Recommendations
-   Implement higher-level timeouts for chunked transfers.
-   Consider async support for better concurrency in high-throughput applications.
