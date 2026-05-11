---
id: TASK-1284
title: >-
  PATTERN-1: single-command and multi-command run paths duplicate
  option-handling
status: Done
assignee:
  - TASK-1305
created_date: '2026-05-11 15:26'
updated_date: '2026-05-11 18:23'
labels:
  - code-review-rust
  - pattern
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/run_cmd.rs:135-174` and `316-350`

**What**: `run_commands` and `run_command` independently destructure `RunOptions`, call `build_runner`, emit dry-run override warnings, and pick the raw-vs-display branch. The two paths use different helpers (`run_command_raw` vs `run_commands_raw`; `run_command_cli` vs `run_commands_with_display`) that internally re-implement the same shape.

**Why it matters**: Past divergence (CL-5/TASK-0755 fixed an inlined tap warning that had drifted from emit_raw_warnings) shows this duplication actively rots. A future flag addition (e.g. --json overriding --raw) must be replicated correctly in two control-flow branches or one path silently no-ops it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 run_command delegates to run_commands with a single-element slice, or both delegate to a shared core function
- [ ] #2 Dry-run warning emission, runtime-kind selection, and raw/display branching exist in exactly one place
- [ ] #3 All existing single- and multi-command tests continue to pass without modification
<!-- AC:END -->
