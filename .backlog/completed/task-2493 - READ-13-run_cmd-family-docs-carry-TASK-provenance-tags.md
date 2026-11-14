---
id: TASK-2493
title: 'READ-13: run_cmd family docs carry TASK provenance tags'
status: Done
assignee: []
created_date: '2026-10-10 15:30'
updated_date: '2026-10-10 21:24'
labels:
  - code-review-rust
  - readability
dependencies: []
parent_task_id: 'TASK-2611'
modified_files:
  - crates/cli/src/run_cmd/explain.rs
  - crates/cli/src/run_cmd/plan.rs
  - crates/cli/src/run_cmd/tools.rs
  - crates/cli/src/run_cmd/dry_run.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:crates/cli/src/run_cmd:module docs'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/run_cmd/explain.rs:2,11,58,61`, `crates/cli/src/run_cmd/plan.rs:6,16`, `crates/cli/src/run_cmd/tools.rs:2,21`, `crates/cli/src/run_cmd/dry_run.rs:78`

**What**: Module and item docs across the run_cmd family open with or embed `TASK-####` tags ("(TASK-2280)", "(TASK-2335)", "One named command's own execution plan (TASK-2262)", "TASK-2275: the plan is a CommandPlan tree, not one flat leaf list", "TASK-2277: a matrix step previews its schedule...").

**Why it matters**: Task ids are process artifacts meaningless to a reader using the code; they go stale while looking authoritative. The repo is stripping these crate by crate (see theme/about/sqlite commits).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 No /// or //! line in the run_cmd family references a TASK id
- [x] #2 Doc summaries describe the end state (what the plan/explain output is), not the change that introduced it

<!-- AC:END -->
