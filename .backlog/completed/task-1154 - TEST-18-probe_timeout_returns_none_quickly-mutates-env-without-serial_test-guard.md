---
id: TASK-1154
title: >-
  TEST-18: probe_timeout_returns_none_quickly mutates env without serial_test
  guard
status: Done
assignee:
  - TASK-1266
created_date: '2026-05-08 07:43'
updated_date: '2026-05-09 14:04'
labels:
  - code-review-rust
  - TEST
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/tools/src/probe.rs:656`

**What**: `timeout_returns_none_quickly` calls `std::env::set_var(TIMEOUT_ENV, \"1\")` and `set_var/remove_var` again on cleanup, but lacks `#[serial_test::serial]`. Sibling tests that mutate `CARGO`/`TIMEOUT_ENV` (tools/tests.rs:953, 989, 1029; deps/tests.rs:1473) are all `#[serial]`-guarded; this one is not.

**Why it matters**: cargo test runs tests in parallel by default. Concurrent env mutation across threads is a documented Rust 2024 hazard (precisely why `set_var` is unsafe) and produces flaky timeouts/false-positives in this test plus collateral failures in any other test that reads OPS_SUBPROCESS_TIMEOUT_SECS.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Add #[serial_test::serial] to timeout_returns_none_quickly
- [x] #2 If the env-mutation pattern repeats, extract a small with_env(key, val, body) helper in a shared test module
<!-- AC:END -->
