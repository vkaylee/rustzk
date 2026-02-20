# Code Review Plan - 2026-02-20

## Overview
This plan addresses issues identified during the codebase review of `rustzk`.

## Phase 1: Performance & Safety (High Priority)
- [ ] **Task 1.1: Optimize `set_user` performance**
  - Modify `set_user` to accept an optional cache.
  - Add `set_users_bulk` method.
- [ ] **Task 1.2: Robust DST handling in `Attendance`**
  - Refactor `timestamp_fixed` to handle `None` and `Ambiguous` results from `chrono` safely.

## Phase 2: Refactoring & Reliability (Medium Priority)
- [ ] **Task 2.1: Improve `read_with_buffer` retries**
  - Add small sleep in empty response loop.
  - Reduce max retries from 100 to a more reasonable number with better logging.
- [ ] **Task 2.2: Optimize `ZKPacket::from_bytes_owned`**
  - Use `Cursor` or `split_off` instead of `drain(0..8)`.

## Phase 3: Documentation & Cleanup (Low Priority)
- [ ] **Task 3.1: Refactor `get_users`**
  - Split into smaller helper functions.
- [ ] **Task 3.2: Expand API Documentation**
  - Add doc comments to all public methods.
