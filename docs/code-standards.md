# Code Standards

## Implementation Rules
- **Reference-First**: Porting logic must refer to `pyzk_ref/` implementation.
- **Safety**: Prefer safe Rust; avoid `unsafe` unless required for FFI (none currently).
- **Errors**: Use the custom `ZKError` enum for all library-specific errors.

## Patterns
- **Newtypes**: Use appropriate types for IDs and session management.
- **Explicit Bitwise**: Use `bitflags` or explicit constants for protocol flags.
- **Documentation**: All public methods must include doc comments explaining the ZK command used.

## Naming
- Structs: PascalCase (`ZK`, `ZKPacket`)
- Methods/Variables: snake_case (`get_attendance`, `session_id`)
- Constants: SCREAMING_SNAKE_CASE (`CMD_CONNECT`)
