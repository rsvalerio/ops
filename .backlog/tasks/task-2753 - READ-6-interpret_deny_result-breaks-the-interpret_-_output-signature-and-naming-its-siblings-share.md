---
id: TASK-2753
title: 'READ-6: interpret_deny_result breaks the interpret_*_output signature and naming its siblings share'
status: To Do
assignee: []
created_date: '2026-11-18 18:41'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - READ
dependencies: []
parent_task_id: 'TASK-2851'
modified_files:
  - extensions-rust/deps/src/parse/deny.rs
  - extensions-rust/deps/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'READ-6:extensions-rust/deps/src/parse/deny.rs:interpret_deny_result'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/parse/deny.rs:100`, `extensions-rust/deps/src/parse/upgrade.rs:56`, `extensions-rust/deps/src/parse/machete.rs:71`

**What**: lib.rs:53-56 describes one pattern for every tool: a `run_cargo_*` entry point that spawns, and an `interpret_*` entry point that applies the format-drift guards to an already-collected `(exit code, output)` triple. Two of the three follow it: `interpret_upgrade_output(exit_code: Option<i32>, stdout: &[u8], stderr: &[u8])` and `interpret_machete_output(exit_code: Option<i32>, stdout: &[u8], stderr: &[u8])`. The deny one differs in both name and shape: `interpret_deny_result(exit_code: Option<i32>, stderr: &str)`. It takes an already lossy-decoded `&str` with no stdout, so `run_cargo_deny` (deny.rs:71) does the `from_utf8_lossy` that the other two do inside their interpret function. The crate comment has to cover it as "`interpret_*_output` / `interpret_*_result`".

**Why it matters**: Three guarded entry points meant to be interchangeable read as two conventions. A caller or test that drives all three tools has to special-case deny, and where the UTF-8 decode happens differs per tool.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The cargo-deny guarded entry point is named and shaped like its siblings (interpret_deny_output taking exit code plus raw stdout/stderr bytes), or the doc comment states why it must differ
- [ ] #2 The lossy UTF-8 decode happens in the same layer for all three tools
- [ ] #3 The ops-deps tests pass
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.15 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
