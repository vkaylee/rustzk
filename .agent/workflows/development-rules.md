# Development Rules - RustZK

## Implementation Strategy
- **Reference-Driven**: All Rust logic should be ported directly from the Python implementation found in `pyzk_ref/`.
- **Protocol Fidelity**: Maintain exact byte-level compatibility with the ZK protocol as defined in `pyzk_ref/zk/base.py`.
- **Data Consistency**: Use the constants defined in `pyzk_ref/zk/const.py` for all command codes and flags.

## Porting Mapping
- `pyzk_ref/zk/base.py` -> `src/lib.rs` and `src/protocol.rs`
- `pyzk_ref/zk/const.py` -> `src/constants.rs`
- `pyzk_ref/zk/user.py`, `attendance.py`, `finger.py` -> `src/models.rs`
