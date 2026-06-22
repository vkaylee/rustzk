# 🧪 Testing Standards

> [!WARNING]
> **NEVER run `cargo test` or `cargo check` directly on the host.**
> This project uses container-based isolation. The ONLY way to run cargo commands is through `./cargo.sh`.

## 1. Test Execution Commands

| Scope | Command | Purpose |
|---|---|---|
| **Compilation check** | `./cargo.sh check` | Validate syntax and type safety |
| **Linter check** | `./cargo.sh clippy` | Run Clippy standard check |
| **Unit & Integration Tests** | `./cargo.sh test` | Run all library test suites |

## 2. Core Testing Principles
- **Pattern:** Use AAA Pattern (Arrange-Act-Assert).
- **Code Coverage:** Enforce thorough coverage on all network protocol parsing, socket connections, and checksum handlers.
- **Mandatory Pass:** A task is NOT complete until all tests pass successfully via `./cargo.sh test`.
