# ⚙️ Configuration Management Rules (ISO 27001 A.8.9)

## 1. Library Configuration Standards (MANDATORY)
- **Centralized Options:** Configuration for ZK device communication (e.g. timeout, passwords, legacy checksum overrides) must be configured through setter methods (e.g. `set_password()`, `set_legacy_checksum()`) or builders rather than global state.
- **Sensible Defaults:** Provide secure, robust default values (e.g. `timeout: Duration::from_secs(60)`, `user_packet_size: 28`).

## 2. Fail-Fast Configuration Validation (MANDATORY)
- **Input Sanitization:** Validate configuration variables (like ports, timeouts, ip addresses) immediately when configuring them. Reject invalid inputs with `ZKError::Connection` or `ZKError::InvalidData` rather than failing later during socket operations.
- **Fail Early:** Validate packet size limits (`validate_packet_size`) prior to allocating memory for incoming buffers to prevent resource exhaustion.

## 3. Test Environment Parity (MANDATORY)
- **Consistent Test Config:** Development/CI test setups must run under the exact same environment variables where applicable.
- **No Hardcoded Test Secrets:** Tests that run against mock/actual devices must read addresses and credentials via environment variables rather than hardcoding them in test code.
