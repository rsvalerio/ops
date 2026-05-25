---
id: TASK-1098
title: >-
  TEST-15: looks_like_secret_value bounded-scan test asserts wall-clock < 1s,
  flaky on shared CI
status: Done
assignee: []
created_date: '2026-05-07 21:33'
updated_date: '2026-05-07 23:19'
labels:
  - code-review-rust
  - tests
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/secret_patterns.rs:222-233`

**What**: The test runs a 2 MiB scan and asserts elapsed `< 1s`. On a saturated CI runner (qemu, sanitiser-instrumented build, shared-tenant Mac runner) a microsecond-scale operation can balloon to seconds. Same flakiness pattern as TASK-1029 (format_error_tail < 50ms) and TASK-1044 (about wrap_text 250ms budget).

**Why it matters**: The bound is generous, but if it fires on CI it should not be blame-able on flake.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace the wall-clock assertion with a syscall/byte-count proxy (e.g. assert the function consults at most SECRET_SCAN_LIMIT bytes via an instrumentation hook)
- [ ] #2 Or mark the test #[ignore] for CI and run only in nightly perf jobs
- [ ] #3 Pin the bound by character iteration count rather than seconds
<!-- AC:END -->
