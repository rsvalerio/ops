---
id: TASK-2034
title: >-
  DRY-1: two CwdGuard implementations — crates/cli's mutex-serialised one and
  the new ops-hook-common test-helper copy
status: To Do
assignee:
  - TASK-2045
created_date: '2026-08-28 23:14'
updated_date: '2026-08-29 11:35'
labels:
  - code-review-rust
  - duplication
dependencies: []
modified_files:
  - extensions/hook-common/src/test_helpers.rs
  - crates/cli/src/test_utils.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/hook-common/src/test_helpers.rs`, `crates/cli/src/test_utils.rs`

**What**: TASK-1908 needed a working-directory guard so `ops-run-before-commit` could test
its production `has_staged_files()` entry point (which reads `std::env::current_dir()`),
and added `CwdGuard` to `ops_hook_common::test_helpers`. `crates/cli/src/test_utils.rs`
already had one. The two are not equivalent:

- the cli copy serialises through a private `CWD_MUTEX` and recovers from poisoning, so
  cwd-dependent cli tests cannot race each other;
- the hook-common copy relies on `#[serial_test::serial]` on each call site instead, which
  is the convention the hook crates already use for `EnvGuard`.

Both are correct for their own suites, but the name is now overloaded across the workspace
and the weaker contract is the one a new hook test will reach for first.

**Why it matters**: DRY-1 — one concept, two implementations with different safety
guarantees and the same name. A future test that copies the hook-common guard into a
non-serial test gets a silent cwd race. `crates/cli` already depends (transitively) on
`ops-hook-common`, so the cli copy could adopt the shared one if the mutex behaviour moves
with it — but that is a cross-crate change with its own blast radius, not a bounded fix
inside a wave scoped to `extensions/run-before-commit/src/lib.rs`.

**Origin**: discovered during TASK-2009 while fixing TASK-1908.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Only one CwdGuard exists in the workspace, or each carries a doc stating why its serialisation contract differs from the other's
- [ ] #2 If the shared guard is the survivor, it keeps the mutex-based serialisation so a non-serial caller cannot race
<!-- AC:END -->
