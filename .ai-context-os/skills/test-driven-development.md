# 🧪 Test-Driven Development (v1.0.0)

> This skill mandates a "Test-First" culture to ensure every change is verified and stable.

## 1. The Red-Green-Refactor Cycle
1. **RED**: Write a failing test first. It must capture the core requirement or bug report.
2. **GREEN**: Write the minimal amount of code needed to pass the test.
3. **REFACTOR**: Clean up the code while keeping the test Green.

## 2. AI Execution Protocol
- When asked to "Add feature X", the first tool call should ideally be creating or updating a test file.
- AI agents must run tests locally before proposing any code changes to the user.

## 3. High-Quality Tests
- **Isolation**: Each test should verify a single behavior.
- **Readability**: Test names should clearly state the intention (e.g., `should-return-error-when-id-is-missing`).
- **NPS (Null, Path, State)**: Always test for null inputs, happy paths, and edge state transitions.
