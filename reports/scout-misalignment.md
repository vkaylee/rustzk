# Scout Report: Request-Response Misalignment

## Exploration Scope
- Target: Protocol Request-Response Misalignment (Sync issue)
- Boundaries: `src/lib.rs`, `src/protocol.rs`

## Patterns Discovered
### Pattern: Sequential Command-Response
- **Location**: `src/lib.rs:201` (`send_command`)
- **Usage**: The current implementation sends a packet and expects exactly one response back from the network stream/socket.
- **Must Follow**: Yes, but needs enhancement to handle out-of-order packets.

## Integration Points
| Point | File | Function | New Code Location |
| ------ | ------ | -------- | ----------------- |
| Command Loop | `src/lib.rs` | `send_command` | Inside the `send_command` match block for Tcp and Udp |

## Conventions
- Error handling: Uses `ZKError` (defined in `lib.rs`).
- Byte ordering: Little-endian for ZK protocol.
- Reply ID: Increments before each send, but not verified upon receive.

## Warnings
- ⚠️ The current implementation of `send_command` for TCP reads a fixed chunk (1040 bytes) and then reads the rest based on the header length. This makes it vulnerable if the header itself is misaligned.
- ⚠️ UDP implementation in `send_command` simply performs a single `recv`, which might catch a delayed packet from a previous command.
