---
id: TASK-0123
title: 'TEST-20: Theme tests set NO_COLOR globally without EnvGuard or #[serial]'
status: Done
assignee: []
created_date: '2026-04-20 19:34'
updated_date: '2026-04-20 20:45'
labels:
  - rust-code-review
  - test-quality
  - concurrency
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/theme/src/tests.rs:30-40` (`NoColorGuard::set`)

**What**: The new `NoColorGuard` calls `std::env::set_var("NO_COLOR", "1")` unconditionally without (a) capturing/restoring the prior value, (b) using the existing `ops_core::test_utils::EnvGuard`, or (c) marking callers `#[serial]`. The SAFETY comment claims "`set_var` is safe under the 2021 edition" and "idempotent writes of the same value make races harmless", but that reasoning is wrong: `setenv` is not thread-safe at the libc level (glibc/musl document it can invalidate pointers returned by `getenv` from other threads), and parallel cargo tests share one process. The existing crate-local helper `EnvGuard` in `crates/core/src/test_utils.rs:385` documents exactly this pitfall and mandates `#[serial]` — the new code bypasses it.

**Why it matters**: (1) Once any other test in the workspace reads `NO_COLOR` (directly or via `apply_style`), two parallel tests touching it races at the libc level, not just the Rust level. (2) The variable persists for the remainder of the process after the guard drops, so ordering-sensitive tests that expect `NO_COLOR` unset silently observe it set. (3) Migration to Rust 2024 will break compilation because `set_var` becomes `unsafe`; the existing `EnvGuard` already wraps that.

**Candidates**:
- `crates/theme/src/tests.rs:38` — `std::env::set_var("NO_COLOR", "1")` in `NoColorGuard::set`
- `crates/theme/src/tests.rs:136` — `label_color_does_not_affect_non_tty_output` uses the guard
- `crates/theme/src/tests.rs:152` — `summary_color_does_not_affect_non_tty_output` uses the guard
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace NoColorGuard with EnvGuard from ops_core::test_utils (or equivalent restoring guard) so prior NO_COLOR value is restored on drop
- [ ] #2 Annotate the two tests (and any future NO_COLOR-dependent test) with #[serial] from serial_test, matching the convention documented on EnvGuard
- [ ] #3 Remove the misleading SAFETY comment claiming set_var is safe / idempotent races are harmless
- [ ] #4 Verify cargo test passes under --test-threads=1 and default parallel execution
<!-- AC:END -->
