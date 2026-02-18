# RustZK - AI Mandates

## Core Rules
- **Validation**: Every modification MUST act as if validated by the following CI steps (Formatting: `cargo fmt --all -- --check`, Building: `cargo build --verbose`, Testing: `cargo test --verbose`, Linting: `cargo clippy -- -D warnings`). Ensure these checks would pass before finalizing changes. Do not run `ci.sh` blindly; execute necessary checks as tool calls.
- **Standards**: Adhere strictly to the rules defined in `.agent/workflows/development-rules.md`.
- **References**: Always consult `pyzk_ref/` for protocol and logic accuracy.
