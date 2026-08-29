---
id: TASK-2034
title: >-
  DRY-1: two CwdGuard implementations — crates/cli's mutex-serialised one and
  the new ops-hook-common test-helper copy
status: Done
assignee:
  - TASK-2045
created_date: '2026-08-28 23:14'
updated_date: '2026-08-29 13:53'
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
- [x] #1 Only one CwdGuard exists in the workspace, or each carries a doc stating why its serialisation contract differs from the other's
- [x] #2 If the shared guard is the survivor, it keeps the mutex-based serialisation so a non-serial caller cannot race
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Resolved in TASK-2045 (wave183).

AC #1: the workspace now has one `CwdGuard`, in `ops_core::test_utils`. The
task named two copies; there was a third — `extensions-rust/deps/src/test_support.rs`,
whose `CwdGuard::set` panicked on failure and had no serialisation at all — so
all three were collapsed. `ops_hook_common::test_helpers` and
`ops_deps::test_support` re-export the shared guard, so
`ops_hook_common::test_helpers::CwdGuard` and `crate::test_support::CwdGuard`
imports are unchanged; `crates/cli` picks it up through the existing
`pub use ops_core::test_utils::*` glob. `ops-deps`'s seven `CwdGuard::set(p)`
call sites became `CwdGuard::new(p).expect("CwdGuard")`, matching the fallible
constructor the other two already used.

AC #2: the survivor is the mutex-based cli implementation, `CWD_MUTEX` and
poisoning recovery included, so a caller that forgets `#[serial_test::serial]`
still cannot race another CWD-dependent test. `ops-hook-common`'s `test-helpers`
feature now turns on `ops-core/test-support` to reach it; `ops-deps` and
`crates/cli` already carried that dev-dependency.

The guard's rustdoc states the contract (mutex-held, not reentrant) at its one
definition site.
<!-- SECTION:NOTES:END -->
