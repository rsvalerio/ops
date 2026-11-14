---
id: TASK-2296
title: 'Add ops backlog wave create --members: create a wave parent and link its members in one step'
status: Done
assignee: []
created_date: '2026-09-26 19:06'
updated_date: '2026-09-26 20:25'
labels:
  - feature
  - backlog
  - waves
dependencies: []
parent_task_id: 'TASK-2305'
modified_files:
  - crates/backlog/src/cmd/wave.rs
  - crates/cli/src/args.rs
  - crates/cli/src/backlog_cmd.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/backlog/src/cmd/wave.rs`

**What**: optional follow-up named in TASK-2285: `ops backlog wave create --members <ids>` should create the wave parent (marker label, dependencies), set each member's `parent_task_id` and flip the members to To Do in one step. Today triage does one `task create` plus N `task edit` calls.

**Why it matters**: removes N+1 separate edits that an agent can get partly wrong (a member missing its parent link or status flip).

**Origin**: discovered during TASK-2291 while fixing TASK-2285 (listed there as an optional follow-up, not an AC).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 wave create --members creates the parent with the marker label and dependencies
- [x] #2 each member gets parent_task_id and status To Do; any failure is reported before partial writes

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Added `ops backlog wave create <title> --members <ids>` (crates/backlog/src/cmd/wave_create.rs; CLI in args.rs/backlog_cmd.rs; docs/backlog.md + README dry-run list). Parent gets marker label + dependencies (status To Do by default); each member gets parent_task_id + --member-status (default To Do). All members are validated (missing / already in a wave / itself a wave) before any write, one error naming every offender; a mid-write I/O failure names linked vs not-linked members. Refused under global --dry-run. run_create core extracted as create_task returning the id.
<!-- SECTION:NOTES:END -->
