---
id: TASK-0070
title: >-
  PERF-3: per-iteration cwd.clone and vars.clone in spawn_parallel_tasks hot
  loop
status: To Do
assignee: []
created_date: '2026-04-17 11:30'
updated_date: '2026-04-17 12:07'
labels:
  - rust-codereview
  - perf
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/mod.rs:434`

**What**: For each parallel task, cwd.clone() and vars.clone() are invoked inside the spawn loop, cloning a PathBuf plus a HashMap<String,String> per task.

**Why it matters**: With many parallel commands the Variables HashMap is cloned repeatedly; wrapping in Arc would let spawned tasks share a single copy cheaply.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Wrap Variables and cwd in Arc once in spawn_parallel_tasks and clone the Arc per task
- [ ] #2 Or document a benchmark showing clone cost is negligible
<!-- AC:END -->
