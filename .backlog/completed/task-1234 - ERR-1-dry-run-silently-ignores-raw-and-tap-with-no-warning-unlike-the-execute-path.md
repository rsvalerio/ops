---
id: TASK-1234
title: >-
  ERR-1: --dry-run silently ignores --raw and --tap with no warning, unlike the
  execute path
status: Done
assignee:
  - TASK-1268
created_date: '2026-05-08 12:58'
updated_date: '2026-05-10 06:35'
labels:
  - code-review-rust
  - errors
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/run_cmd.rs:149-168`

**What**: `run_commands` routes early into the dry-run branch when `opts.dry_run` is set, returning Ok(SUCCESS) without consulting `opts.raw` or `opts.tap`. The execute path emits `emit_raw_warnings` ("--raw forces sequential", "--tap is ignored under --raw") for the same conflicts; dry-run never does.

**Why it matters**: A user invoking `ops <cmd> --dry-run --raw --tap=path` gets neither a warning nor any sign that --raw/--tap had no effect. The execute path's READ-10 warning contract does not extend to dry-run, despite the same flag ambiguity.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Call emit_raw_warnings (or a dry-run equivalent) when dry_run is set with raw/tap flags
- [ ] #2 OR document in clap's --dry-run help that it overrides --raw/--tap
- [ ] #3 Unit test asserting the warn fires on --dry-run --raw
<!-- AC:END -->
