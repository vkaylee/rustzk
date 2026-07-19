# Execution Wrappers & Hermetic Execution (CRITICAL)

> **MANDATORY:** AI Agents MUST use these wrapper scripts instead of raw commands (e.g., `cargo`) to ensure hermetic execution within the container.

## 🚫 The Problem: Host Environment Pollution & Missing Cargo
If you run `cargo test` or `cargo check` directly on the host machine, it will:
1. Fail because cargo is not installed on the host.
2. Break the hermetic environment (using host tools instead of the correct container versions).

## ✅ The Solution: Orchestrator Wrapper Scripts
You MUST use the provided wrapper script `./cargo.sh` located in the root directory for ALL cargo commands. This script proxies your command into the correct container using `compose.test.yml`.

| Action / Tool | ❌ INCORRECT (DO NOT USE) | ✅ CORRECT (MUST USE) |
|---|---|---|
| **Rust / Cargo** | `cargo test`, `cargo build`, `cargo check` | `./cargo.sh test`, `./cargo.sh build`, `./cargo.sh check` |
| **Full CI Pipeline** | `./cargo.sh test` (unit tests only, skips integration + lint) | `./leedevkit test rust` (unit + integration + lint) |
| **Quick check** | `cargo check`, `cargo clippy`, `cargo fmt` | `./cargo.sh check`, `./cargo.sh clippy`, `./cargo.sh fmt` |

## ⚠️ PTY Safety & Redirection (MANDATORY)
When running commands asynchronously in the background, you **MUST ALWAYS** redirect the output to a log file to prevent PTY hangs.

**Rule:** `> [logfile] 2>&1 </dev/null`

**Examples:**
- `./cargo.sh test > test.log 2>&1 </dev/null`
- `./cargo.sh clippy > clippy.log 2>&1 </dev/null`

After executing the command, check the output or read the log file to report results.
