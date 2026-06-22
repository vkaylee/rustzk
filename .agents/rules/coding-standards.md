# 💻 Coding Standards & Best Practices

## 1. Language Handling
- **Communication:** Respond in the language the user initiated the prompt with.
- **Code:** ALL variables, function names, and code comments MUST be in English.

## 2. Clean Code Standards
- **SOLID & DRY:** Enforce Single Responsibility. Extract reusable logic.
- **Self-Documenting:** Write clear names. Minimal comments. No over-engineering.
- **Strict Encapsulation (OOP Style):** ALL struct fields MUST be `private` by default to ensure strict data protection and integrity.
  - Do NOT use `pub` for struct fields unless absolutely necessary.
  - State mutation and data access MUST be controlled via explicitly defined `new()` constructors and getter/setter methods.

## 3. Error Handling & Observability
- **Structured Errors:** Use domain-specific error types (`ZKError` and `ZKResult`). Never throw generic `Error`.
- **Zero Panic Mindset:** NEVER call `unwrap()`, `expect()`, or `panic!` inside library logic. Always propagate errors gracefully.
- **Telemetry Baseline:** Incorporate structured logging (`log::debug`, `log::info`) to trace low-level network packet exchanges.

## 4. Security & Compliance
- **Data Privacy:** NEVER log Personally Identifiable Information (PII) like raw passwords or authentication communication keys in plain text.
- **Secrets:** NEVER hardcode communication passwords or device secrets. Always pass them dynamically.

## 5. Boy Scout Rule
If you see a bug, typo, or poor pattern nearby while working on a file, FIX IT IMMEDIATELY.
> 🔴 **NEVER say "this is out of scope for the current task" when a clear bug is visible.**

## 6. Enterprise-Grade Premium Standard (MANDATORY)
> 🔴 **MANDATORY:** Every single feature, component, and utility developed in this workspace MUST default to an **Enterprise-Grade Premium Ready** standard. Do NOT deliver MVP-style or basic implementations.
- **Protocol Safety:** Validate packet bounds strictly (`validate_packet_size`) to prevent buffer overflow or denial-of-service/memory-exhaustion attacks.

## 7. Performance & Resource Optimization
- **Buffer Reuse:** Avoid per-packet heap allocations. Utilize reusable buffers (e.g., `udp_buf`) for network socket read/write operations.
- **Buffer Size Tuning:** Ensure UDP socket receive buffers are sized appropriately (e.g. via `socket2` to 2MB) to prevent packet loss under high-volume transfer (e.g. logs sync).

## 8. Escalation Protocol
- **The Rule:** If an error persists after **2 failed self-correction attempts**, STOP IMMEDIATELY. Generate an Incident Report and ask for human intervention. Do not endlessly retry.
