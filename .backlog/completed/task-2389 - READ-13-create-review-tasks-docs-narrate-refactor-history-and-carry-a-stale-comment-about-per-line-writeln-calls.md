---
id: TASK-2389
title: 'READ-13: create-review-tasks docs narrate refactor history and carry a stale comment about per-line writeln! calls'
status: Done
assignee: []
created_date: '2026-10-04 14:15'
updated_date: '2026-10-04 15:59'
labels:
  - code-review-rust
  - READ
dependencies: []
parent_task_id: 'TASK-2422'
modified_files:
  - extensions/create-review-tasks/src/backlog.rs
  - extensions/create-review-tasks/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions/create-review-tasks/src/backlog.rs:next_ids'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/create-review-tasks/src/backlog.rs:73`, `:140`, `:166`, `:543`; `extensions/create-review-tasks/src/lib.rs:48`, `:448`, `:927`

**What**: Docs and comments describe the journey rather than the end state. `next_ids` says "the two allocators used to be the same scan-extract-max loop run twice" (backlog.rs:73), and a test doc says "separate walks used to" (backlog.rs:543). Rule-ID/ticket tags such as `PERF-3 / TASK-2131`, `PERF-13 / TASK-2117` and `API-2 / TASK-2114` stand in for rationale (backlog.rs:140, 166; lib.rs:48, 448, 927). The `stage_task_file` comment at lib.rs:448 is also stale: it justifies the `BufWriter` by "the ~14 `writeln!` calls in `render_task_file`", but `render_task_file` now emits one `write_all` of the rendered `TaskDoc` (backlog.rs:215), so the stated reason no longer exists.

**Why it matters**: Text a later reader would delete verbatim goes stale while looking authoritative, and lib.rs:448 already has. Ticket IDs are meaningless to API readers.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Docs and comments in create-review-tasks state current behaviour without 'used to' history or TASK-NNNN references
- [x] #2 The BufWriter comment in stage_task_file reflects that render_task_file performs a single write_all, or is dropped

<!-- AC:END -->
