---
id: TASK-2375
title: 'READ-1: Signal-kill error message in check_llvm_cov_output states ''terminated by signal'' twice'
status: Triage
assignee: []
created_date: '2026-10-04 14:12'
labels:
  - code-review-rust
  - READ
dependencies: []
modified_files:
  - extensions-rust/test-coverage/src/subprocess.rs
  - extensions-rust/test-coverage/src/tests/subprocess.rs
priority: low
ordinal: 1000
dedup_key: 'READ-1:extensions-rust/test-coverage/src/subprocess.rs:check_llvm_cov_output'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/test-coverage/src/subprocess.rs:134-137` (`check_llvm_cov_output`)

**What**: On the signal arm the bail is `"cargo llvm-cov terminated by signal ({marker}): {tail}"` where `marker` is `format_cargo_exit(status)` = `"exit_code = None (terminated by signal)"`. The rendered error reads `cargo llvm-cov terminated by signal (exit_code = None (terminated by signal)): ...`, repeating the phrase and nesting parentheses. The `Some(_)` arm also discards the code it matched and re-derives it via `marker`. No test in `src/tests/subprocess.rs` pins the signal-arm text, so the shape is unguarded.

**Why it matters**: Operator-facing diagnostic for the SIGKILL/OOM case is noisy and hard to grep; the comment promises a stable, greppable shape (TASK-1099/1560) but the signal arm is not covered by any assertion.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Signal-kill error mentions the signal termination once, keeping the 'cargo llvm-cov' prefix and the stderr tail
- [ ] #2 A unix test asserts the signal-arm message (via ExitStatus::from_raw(9)) so the shape is pinned
<!-- AC:END -->
