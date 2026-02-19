# ADR 003: Protocol Robustness and Centralized Packet Reading

## Status
Accepted

## Context
The ZK protocol relies on a request-response model where each packet carries a `reply_id`. However, the underlying transport (TCP/UDP) can exhibit behaviors like fragmentation, coalescing, or out-of-order delivery (UDP). The initial implementation duplicated packet reading logic across `send_command` and `receive_chunk`, leading to inconsistent handling of `reply_id` validation and potential desynchronization, especially with chunked data.

## Decision
We have decided to:
1.  **Centralize Packet Reading**: Create a private `read_packet` helper method in `ZK` struct. This method is the *single authority* for reading a ZK packet from the network.
2.  **Enforce Strict Validation**: `read_packet` must validate `reply_id` against the expected session `reply_id`. Mismatched packets are discarded.
3.  **Implement Safety Valves**: To prevent infinite loops during validation, `read_packet` enforces a `MAX_DISCARDED_PACKETS` limit (set to 100).
4.  **Use `read_exact` for TCP**: To prevent over-reading and desynchronization, TCP stream reading must strictly use `read_exact` for the fixed header and the variable-length body.

## Consequences
### Positive
-   **Reliability**: Drastically reduced chance of protocol desynchronization.
-   **Security**: Protected against infinite loops and malicious packet injection.
-   **Maintainability**: Single point of change for packet reading logic.

### Negative
-   **Performance**: Slight overhead from checking `reply_id` and discard limits (negligible in practice).

## Compliance
All future transport logic changes must use `read_packet` or strictly adhere to its validation patterns.
