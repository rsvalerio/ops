---
id: TASK-2871
title: 'CONC-16: test-support helpers still recover through poisoned mutexes'
status: Triage
assignee: []
created_date: '2026-11-18 21:53'
labels:
  - code-review-rust
  - CONC
dependencies: []
modified_files:
  - crates/core/src/test_utils.rs
  - crates/cli/src/test_utils.rs
  - crates/extension/tests/public_api.rs
  - extensions-rust/loc/src/tests.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/test_utils.rs:814` (`CwdGuard::new`), `crates/core/src/test_utils.rs:872-879` (capture writer), `crates/cli/src/test_utils.rs:122` (`cwd_guard_mutex_is_recoverable`), `crates/extension/tests/public_api.rs:1668-1939`, `extensions-rust/loc/src/tests.rs:834-849`

**What**: After TASK-2839 moved every production lock to the propagate-on-poison policy (`ops_core::sync::lock`), these test-support and test-only sites still call `PoisonError::into_inner`. `CwdGuard` is behind the `test-support` feature and deliberately recovers `CWD_MUTEX` so one panicking cwd test does not cascade into every other cwd test under `cargo test`.

**Why it matters**: CONC-16 targets production locks; the test-support sites need an explicit decision. Either they adopt `ops_core::sync::lock` (relying on nextest's process-per-test isolation), or the exemption for test infrastructure is recorded next to the code.

**Origin**: discovered during TASK-2839 while fixing TASK-2672.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each listed test-support/test-only site either propagates poison via ops_core::sync::lock (or .expect) or carries a comment recording the test-infrastructure exemption
<!-- AC:END -->
