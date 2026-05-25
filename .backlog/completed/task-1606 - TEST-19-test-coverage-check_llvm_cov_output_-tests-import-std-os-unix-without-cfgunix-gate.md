---
id: TASK-1606
title: >-
  TEST-19: test-coverage check_llvm_cov_output_* tests import std::os::unix
  without cfg(unix) gate
status: Done
assignee:
  - TASK-1635
created_date: '2026-05-22 06:43'
updated_date: '2026-05-22 10:13'
labels:
  - code-review-rust
  - test
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/test-coverage/src/tests.rs:332-374`

**What**: Three tests — `check_llvm_cov_output_success` (line 333), `check_llvm_cov_output_failure_includes_stderr_tail` (line 344), and `check_llvm_cov_output_failure_empty_stderr` (line 363) — call `use std::os::unix::process::ExitStatusExt;` and construct `ExitStatus::from_raw(...)` without any `#[cfg(unix)]` attribute. Two sister tests in the same module (`check_llvm_cov_output_failure_includes_exit_code` at line 380 and `check_llvm_cov_output_failure_signal_kill_says_signal` at line 397) are correctly gated with `#[cfg(unix)]`, demonstrating the intended pattern.

**Why it matters**: The `std::os::unix` module does not exist on Windows targets; the three ungated tests will fail `cargo test --target x86_64-pc-windows-*` with a hard compile error, not just a runtime skip. The asymmetry inside the same file (some `#[cfg(unix)]`, some not) shows the gate was added retroactively to the panic / signal-kill tests but the original three were missed. This is the exact pattern called out in TEST-19 / TASK-1591 for the tools crate.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 All three tests (`check_llvm_cov_output_success`, `check_llvm_cov_output_failure_includes_stderr_tail`, `check_llvm_cov_output_failure_empty_stderr`) are annotated with `#[cfg(unix)]` matching their sister tests
- [ ] #2 `cargo check --target x86_64-pc-windows-msvc --tests -p ops-test-coverage` (or equivalent cross-check) no longer fails on `std::os::unix` resolution for these tests
- [ ] #3 If a Windows-portable variant is feasible (e.g. using a real subprocess that exits non-zero), document why `#[cfg(unix)]` was retained rather than replaced
<!-- AC:END -->
