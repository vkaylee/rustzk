# Contributing to RustZK

First off, thank you for considering contributing to RustZK! It's people like you that make the open-source community such a great place.

## Development Environment

1. **Rust**: You'll need the latest stable version of Rust installed.
2. **Tools**: Ensure you have `rustfmt` and `clippy` installed.
   ```bash
   rustup component add rustfmt clippy
   ```

## Development Workflow

1. **Fork the repo** and create your branch from `main`.
2. **Implement your changes**.
3. **Write tests** for any new functionality or bug fixes.
4. **Run the CI script** locally to ensure everything is correct:
   ```bash
   ./scripts/ci.sh
   ```
5. **Formatting**: Always run `cargo fmt --all` before committing.

## Coding Standards

- **Sync vs Async**: Most new features should be implemented for both the synchronous `ZK` client and the asynchronous `ZKAsync` client to maintain parity.
- **Protocol Fidelity**: If you are adding new protocol commands, refer to the `pyzk_ref` directory (the reference Python implementation) to ensure byte-level compatibility.
- **Error Handling**: Use the `ZKError` enum for all public-facing errors.
- **Documentation**: All public methods and structs must have comprehensive doc comments.

## Protocol Commands

Command constants are located in `src/constants.rs`. If you discover new command codes, please add them there with descriptive names.

## Pull Request Process

1. Ensure your PR passes all CI checks.
2. Keep your PR focused. If you want to contribute multiple unrelated features, please open separate PRs.
3. Update `CHANGELOG.md` with your changes under the `[Unreleased]` section (or create it if it doesn't exist).

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
