---
id: TASK-2530
title: 'FN-1: run_commit spans ~105 lines, mixing four numbered orchestration steps inline'
status: Done
assignee: []
created_date: '2026-10-10 15:34'
updated_date: '2026-10-10 21:31'
labels:
  - code-review-rust
  - functions
dependencies: []
parent_task_id: 'TASK-2613'
modified_files:
  - crates/backlog/src/cmd/commit.rs
priority: medium
ordinal: 1000
dedup_key: 'FN-1:crates/backlog/src/cmd/commit.rs:run_commit'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/backlog/src/cmd/commit.rs:50` (run_commit, ~105 raw lines to line 155)

**What**: run_commit exceeds the 50-line FN-1 threshold 2x. Its body already names its abstraction levels in comments — "Step 2: the index must hold nothing but our own files", "Step 3: only the task files with a change ... are committed", "Step 4" — i.e. the steps are begging to be functions: an index-purity check, a changed-paths computation (porcelain parse), and the add/commit-with-rollback sequence.

**Why it matters**: The commit-criticality logic (refuse-foreign-index, rollback on failed commit) is buried mid-function; a reader must hold four steps to verify the "touch nothing on refusal" invariant. Extracting each numbered step into a named helper makes the invariant checkable step by step.

<!-- scan confidence: raw-line measurement including comments; renderer/parser functions with the same span were excluded per FN-1 exceptions -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 run_commit orchestrates named helpers, one per documented step, and itself stays under ~50 lines
- [x] #2 Refusal-on-foreign-index and rollback-on-failed-commit behaviour is unchanged (existing tests pass)

<!-- AC:END -->
