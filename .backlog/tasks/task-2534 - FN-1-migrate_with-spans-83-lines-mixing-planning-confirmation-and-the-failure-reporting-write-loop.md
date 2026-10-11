---
id: TASK-2534
title: 'FN-1: migrate_with spans ~83 lines mixing planning, confirmation, and the failure-reporting write loop'
status: Done
assignee: []
created_date: '2026-10-10 15:34'
updated_date: '2026-10-10 21:33'
labels:
  - code-review-rust
  - functions
dependencies: []
parent_task_id: 'TASK-2613'
modified_files:
  - crates/backlog/src/cmd/wave.rs
priority: medium
ordinal: 1000
dedup_key: 'FN-1:crates/backlog/src/cmd/wave.rs:migrate_with'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/backlog/src/cmd/wave.rs:379` (migrate_with, ~83 raw lines to line 462)

**What**: migrate_with exceeds the 50-line FN-1 threshold. It inlines: wave discovery, preflight, plan reporting, dry-run/confirmation gating, and the write loop whose error arm builds an already-written summary string inline (lines 437-451).

**Why it matters**: The write loop's failure reporting is the repair path for a stopped migration and is buried as a multi-line closure inside `with_context`. Extracting the report/confirm phase and the already-written-summary formatting lets each piece fit on one screen.

<!-- scan confidence: raw-line measurement including comments -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 migrate_with's confirmation phase and write-loop error summary are named helpers; the function stays under ~50 lines
- [x] #2 Preflight-before-write and stopped-migration reporting behaviour is unchanged (existing tests pass)

<!-- AC:END -->
