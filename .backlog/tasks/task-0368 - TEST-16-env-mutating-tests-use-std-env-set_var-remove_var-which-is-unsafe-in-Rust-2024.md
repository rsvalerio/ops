---
id: TASK-0368
title: >-
  TEST-16: env-mutating tests use std::env::set_var/remove_var which is unsafe
  in Rust 2024
status: To Do
assignee:
  - TASK-0421
created_date: '2026-04-26 09:37'
updated_date: '2026-04-26 10:10'
labels:
  - code-review-rust
  - test-quality
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/run-before-commit/src/lib.rs:104` (also run-before-push and hook-common)

**What**: EnvGuard calls std::env::set_var / remove_var which become unsafe in 2024 edition because they race with concurrent getenv. Pattern duplicated across run-before-push, run-before-commit, and hook-common.

**Why it matters**: Future edition migration will turn these into hard compile errors; today they remain a flakiness vector if any other test reads env without serial_test::serial.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Centralize EnvGuard in hook-common (or a test-helper crate) and gate it appropriately for upcoming editions
- [ ] #2 All call sites import the shared guard; per-crate copies removed
<!-- AC:END -->
