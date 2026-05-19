---
id: TASK-1564
title: >-
  DUP-3: tools::probe cargo.rs and rustup.rs duplicate
  spawn/non-zero/stderr-snippet scaffolding across four functions
status: To Do
assignee:
  - TASK-1578
created_date: '2026-05-19 15:56'
updated_date: '2026-05-19 16:46'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/tools/src/probe/cargo.rs:59`, `extensions-rust/tools/src/probe/cargo.rs:98`, `extensions-rust/tools/src/probe/rustup.rs:59`, `extensions-rust/tools/src/probe/rustup.rs:138`

**What**: `check_cargo_tool_installed` / `check_rustup_component_installed` share an identical "run_probe_with_timeout -> match Failed -> if !status.success() { tracing::warn!(stderr=?format_error_tail(...,10)); return Failed } -> from_utf8_lossy(...)" body. `capture_cargo_list` / `capture_rustup_components` share the same spawn -> run_probe -> non-zero -> `from_utf8_lossy(...).into_owned()` body. Each pair is ~15-20 lines of structural copy.

**Why it matters**: Adding a new probe (e.g. a future `rustup target list` capture) requires copy-pasting the same scaffold, and any policy change (e.g. stderr snippet line count, ProbeFailed reporting nuance) has to be repeated in two/four places. The `from_utf8_lossy(...).into_owned()` also allocates unconditionally even for valid UTF-8 (PERF-3 nuance from TASK-1232 applies).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extract a run_probe_capturing helper (in probe/timeout.rs or probe/mod.rs) returning ProbeOutcome<String> from a Command, folding non-zero-exit + stderr_snippet warn into one place
- [ ] #2 check_cargo_tool_installed and check_rustup_component_installed reuse the helper; only their membership predicate remains in the per-probe body
- [ ] #3 capture_cargo_list and capture_rustup_components call the helper directly
- [ ] #4 No behavioural change in is_installed semantics; existing tests stay green
<!-- AC:END -->
