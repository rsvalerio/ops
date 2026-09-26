---
id: TASK-2296
title: 'Add ops backlog wave create --members: create a wave parent and link its members in one step'
status: Triage
assignee: []
created_date: '2026-09-26 19:06'
labels:
  - feature
  - backlog
  - waves
dependencies: []
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
- [ ] #1 wave create --members creates the parent with the marker label and dependencies
- [ ] #2 each member gets parent_task_id and status To Do; any failure is reported before partial writes
<!-- AC:END -->
