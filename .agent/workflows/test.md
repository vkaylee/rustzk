---
description: PTY-safe test runner — cleans stale locks before running cargo test
---
# Run Tests (PTY-Safe)

// turbo-all

1. Kill any zombie cargo processes and clean stale locks:
```bash
pkill -f "cargo test" 2>/dev/null || true; pkill -f "cargo check" 2>/dev/null || true; rm -f target/.cargo-lock 2>/dev/null; echo "CLEANED"
```

2. Run all tests:
```bash
timeout 120 cargo test --no-fail-fast 2>&1; echo "TEST_EXIT=$?"
```
