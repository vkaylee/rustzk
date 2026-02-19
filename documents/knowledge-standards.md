# Coding Standards: rustzk

## Naming Conventions
- **Rust Standard:** Snake case for variables and functions (`my_variable`), CamelCase for structs and enums (`MyStruct`).
- **Constants:** Upper snake case (`MAX_BUFFER_SIZE`).
- **Files:** Snake case (`client_tests.rs`).

## Code Style
- **Indentation:** 4 spaces.
- **Max Line Length:** 100 characters (soft limit, 120 hard limit).
- **Comments:** Use `///` for documentation comments on public items.
- **Safety:** Avoid `unsafe` blocks unless absolutely necessary.
- **Error Handling:** Use `ZKResult` for functions that can fail.

## Testing
- **Unit Tests:** Located in the same file as the code (`#[cfg(test)] mod tests { ... }`).
- **Integration Tests:** Located in the `tests/` directory.
- **Mocking:** Use mock servers (like in `tests/client_tests.rs`) to simulate device behavior.

## Commit Messages
- **Format:** `type(scope): subject`
- **Types:**
    - `feat`: New feature.
    - `fix`: Bug fix.
    - `docs`: Documentation changes.
    - `style`: Formatting, missing semicolons, etc.
    - `refactor`: Code change that neither fixes a bug nor adds a feature.
    - `test`: Adding missing tests or correcting existing tests.
    - `chore`: Infrastructure changes (e.g., CI/CD).
- **Example:** `feat(protocol): add TCP fragmentation support`

## Pull Requests
- **Title:** Clear and concise, following commit message format.
- **Description:** Explain *what* was changed and *why*.
- **Review:** All PRs must be reviewed by at least one other developer.
- **CI:** All tests must pass before merging.
