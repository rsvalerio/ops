---
id: TASK-1099
title: >-
  PATTERN-1: check_metadata_output / check_llvm_cov_output omit exit code from
  error message, breaking parity with sister helpers
status: Done
assignee: []
created_date: '2026-05-07 21:33'
updated_date: '2026-05-08 06:52'
labels:
  - code-review-rust
  - pattern
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/lib.rs:39-45` and `extensions-rust/test-coverage/src/lib.rs:109-115`

**What**: Both helpers bail with `"cargo X failed: {tail}"` but never include `output.status.code()`. The sibling functions in `deps/src/parse.rs` (`interpret_upgrade_output`, `interpret_deny_result`) deliberately surface the exit code so operators can distinguish "exit 1 (issues found)" from "exit 101 (panic)" from "None (signal)".

**Why it matters**: A SIGKILL/OOM kill versus a real cargo failure produces the same error string, masking infrastructure issues as workspace bugs.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 check_metadata_output and check_llvm_cov_output include the numeric exit code (or 'signal' for None) in the bailed error
- [x] #2 A unit test pins the exit-code substring for at least one non-zero code and one signal-killed case
- [x] #3 Pattern matches the format used by interpret_deny_result (e.g. 'cargo X exited with status {code}: {tail}')
<!-- AC:END -->
