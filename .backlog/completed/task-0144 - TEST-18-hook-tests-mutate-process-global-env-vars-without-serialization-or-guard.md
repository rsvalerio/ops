---
id: TASK-0144
title: >-
  TEST-18: hook tests mutate process-global env vars without serialization or
  guard
status: Done
assignee: []
created_date: '2026-04-22 21:22'
updated_date: '2026-04-23 07:38'
labels:
  - rust-code-review
  - TEST
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Files**:
- `extensions/run-before-commit/src/lib.rs:110` (`should_skip_returns_false_by_default` calls `std::env::remove_var(SKIP_ENV_VAR)`)
- `extensions/run-before-push/src/lib.rs:97` (same pattern with `SKIP_OPS_RUN_BEFORE_PUSH`)
- `extensions/hook-common/src/lib.rs:224` (`should_skip_returns_false_by_default` mutates env without guard)

**What**: Tests call `std::env::remove_var` directly. `cargo test` runs tests within a binary in parallel threads by default. Env vars are process-global, so any concurrent test that reads/writes these vars (or a future test that sets them) can race. Also, on Rust 2024 edition `std::env::remove_var`/`set_var` become `unsafe` (UNSAFE-8) — flagging early.

**Why it matters**: Flaky tests under parallel execution; blocks future edition migration. Prior art exists in this repo: TASK-0123 solved this for theme tests with an `EnvGuard`/`#[serial]` approach.

<!-- scan confidence: candidates to inspect -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Serialize env-mutating hook tests (e.g. serial_test crate or shared EnvGuard)
- [x] #2 Restore previous env var value in Drop so one test cannot leak state into another
<!-- AC:END -->
