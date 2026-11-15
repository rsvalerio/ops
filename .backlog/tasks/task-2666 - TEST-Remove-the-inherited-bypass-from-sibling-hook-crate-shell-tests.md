---
id: TASK-2666
title: 'TEST: Remove the inherited bypass from sibling hook-crate shell tests'
status: Triage
assignee: []
created_date: '2026-11-15 10:12'
labels:
  - code-review-rust
  - test
dependencies: []
modified_files:
  - extensions/run-before-commit/src/lib.rs
  - extensions/hook-common/src/lib.rs
priority: medium
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/run-before-commit/src/lib.rs:241`, `extensions/hook-common/src/lib.rs:349`

**What**: The shell-test `Command` constructors in `run-before-commit` (`hook_script_reports_a_missing_ops_binary_by_name`, `hook_script_honours_the_bypass_when_ops_is_missing`) and in `hook-common` (`hook_script_prologue_passes_sh_n_bypass_and_fails_closed`'s `run` closure) inherit the parent environment without removing their hook's bypass variable. `Command` inherits env vars it does not override, so `SKIP_OPS_RUN_BEFORE_COMMIT=1 cargo test -p ops-run-before-commit` (or `SKIP_OPS_RUN_BEFORE_PROBE=1` for hook-common) makes the hook exit 0 before the missing-ops probe: the fail-closed assertions then see success instead of exit 1 and the suite fails, although the hook behaves correctly.

**Why it matters**: Same defect TASK-2664 fixed for `run-before-push` — test suites must stay deterministic under env leakage. A configured check or developer shell that exports the bypass (its advertised usage) breaks these tests.

**Origin**: discovered during TASK-2665 while fixing TASK-2664. Fix shape: `.env_remove(<SKIP_ENV_VAR>)` on each constructor, with any explicit per-test `envs` applied afterward so bypass-selection tests still work.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every shell-test Command constructor in run-before-commit and hook-common removes its hook's bypass env var, with explicit test envs still applied after the removal
- [ ] #2 Suites pass under SKIP_OPS_RUN_BEFORE_COMMIT=1 and SKIP_OPS_RUN_BEFORE_PROBE=1 respectively
<!-- AC:END -->
