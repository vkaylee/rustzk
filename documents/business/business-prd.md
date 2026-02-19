# Product Requirements Document (PRD): rustzk

## 1. Introduction
`rustzk` is a high-performance, open-source library that enables seamless communication with ZKTeco time and attendance devices. It aims to replace legacy SDKs with a modern, type-safe Rust implementation.

## 2. Goals
- **Independence:** Eliminate reliance on vendor-provided Windows-only SDKs.
- **Reliability:** Ensure stable connections and accurate data retrieval under various network conditions.
- **Performance:** Handle large volumes of attendance logs efficiently.
- **Usability:** Provide a clean, idiomatic Rust API for developers.

## 3. Target Audience
- System Integrators building HR/Payroll systems.
- IoT Developers creating smart building solutions.
- DevOps Engineers managing device fleets.

## 4. User Stories
### Integration Developer
> As a developer, I want to fetch attendance logs from a device programmatically so that I can sync them with my company's payroll system.

### System Administrator
> As an admin, I want to synchronize the time on all devices to a central server time so that attendance records are accurate.

### Security Officer
> As a security officer, I want to disable access for terminated employees immediately by removing their user data from the device.

## 5. Functional Requirements
- **FR1:** Connect to devices via TCP (default) and UDP.
- **FR2:** Retrieve user list (ID, name, card number).
- **FR3:** Retrieve attendance logs with precise timestamps (ISO-8601).
- **FR4:** Support timezone synchronization.
- **FR5:** Manage device state (enable/disable, restart, clear data).
- **FR6:** Real-time event monitoring (future scope).

## 6. Non-Functional Requirements
- **NFR1:** Must run on Linux, macOS, and Windows.
- **NFR2:** Zero external runtime dependencies (pure Rust).
- **NFR3:** Memory safe and thread-safe.
- **NFR4:** Error handling must be explicit and descriptive.
