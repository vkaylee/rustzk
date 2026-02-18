#!/bin/bash
set -e

echo "--- 1. Checking formatting ---"
cargo fmt --all -- --check

echo "--- 2. Building project ---"
cargo build --verbose

echo "--- 3. Running all tests ---"
cargo test --verbose

echo "--- 4. Running linter (clippy) ---"
cargo clippy -- -D warnings

echo "--- CI PASSED ---"
