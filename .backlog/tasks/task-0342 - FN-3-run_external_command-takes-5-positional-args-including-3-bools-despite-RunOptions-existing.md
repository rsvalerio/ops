---
id: TASK-0342
title: >-
  FN-3: run_external_command takes 5 positional args including 3 bools despite
  RunOptions existing
status: Done
assignee:
  - TASK-0420
created_date: '2026-04-26 09:34'
updated_date: '2026-04-27 11:26'
labels:
  - code-review-rust
  - structure
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/run_cmd.rs:38-52` and `crates/cli/src/subcommands.rs:151-152`

**What**: run_external_command(args, dry_run, verbose, tap, raw) exposes three adjacent bool parameters; RunOptions already exists in the same file (line 31). Callers like subcommands.rs:152 invoke run_external_command(&args, false, false, None, false).

**Why it matters**: The struct fix is half-done — internally rebuilds RunOptions but public signature still accepts bools positionally. Future fields perpetuate the same boolean parade.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Change run_external_command (and run_command/run_command_raw/run_command_cli chain) to take RunOptions instead of individual bools
- [ ] #2 Update call sites in main.rs and subcommands.rs to construct RunOptions explicitly; ensure cargo clippy and tests pass
<!-- AC:END -->
