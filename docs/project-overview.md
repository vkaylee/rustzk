# Project Overview - RustZK

## Objective
Port the `pyzk2` Python library to Rust to provide a safe, fast, and concurrent solution for ZKTeco device integration.

## Roadmap
- [x] Core Protocol Implementation (Checksum, Packet wrapping)
- [x] Basic Connection (TCP/UDP)
- [x] Device Info & Capacity
- [x] Attendance & User Retrieval
- [x] Comprehensive Test Suite with Device Emulator
- [x] GitHub Actions CI/CD Pipeline
- [ ] User Management (Add/Delete/Update)

## Requirements
- Rust 1.70+
- Dependencies: `chrono`, `byteorder`, `thiserror`, `bitflags`
