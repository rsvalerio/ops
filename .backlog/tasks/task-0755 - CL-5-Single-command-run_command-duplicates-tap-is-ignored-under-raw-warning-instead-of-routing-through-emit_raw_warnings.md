---
id: TASK-0755
title: >-
  CL-5: Single-command run_command duplicates --tap is ignored under --raw
  warning instead of routing through emit_raw_warnings
status: Done
assignee:
  - TASK-0825
created_date: '2026-05-01 05:53'
updated_date: '2026-05-01 11:41'
labels:
  - code-review-rust
  - structure
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/run_cmd.rs:243-248`

**What**: run_commands_raw (line 142) calls emit_raw_warnings(any_parallel, tap.is_some()) which centralises both warnings. The single-command run_command path inlines a copy of the tap warning with the identical message string and delegates parallel-ignored to warn_raw_drops_parallel.

**Why it matters**: Two divergent code paths emitting the same warning have already drifted. A future tweak will be applied in one path but not the other.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Single-command path emits both warnings via emit_raw_warnings (or a shared helper) so the message string lives in one place
- [ ] #2 Unit test invokes the single-command raw path with tap = Some(...) and asserts the warning is emitted exactly once
- [ ] #3 Editing the literal warning string in emit_raw_warnings is sufficient — no second site needs synchronised editing
<!-- AC:END -->
