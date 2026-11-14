---
id: TASK-2336
title: 'SEC-13: deps extension registers a bare ''ops'' program resolved through PATH instead of current_ops_program'
status: Done
assignee: []
created_date: '2026-09-28 15:19'
updated_date: '2026-09-28 16:55'
labels:
  - code-review-rust
  - security
dependencies: []
parent_task_id: 'TASK-2340'
modified_files:
  - extensions-rust/deps/src/lib.rs
priority: medium
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/lib.rs:436`

**What**: `register_commands` inserts `ExecCommandSpec::new("ops", ["deps"])`, so `ops qa` (whose first step is `deps`) spawns whatever `ops` is first on PATH rather than the running binary. TASK-2122/TASK-2255 migrated check-json/check-yaml, text-fixers and about to `ExecCommandSpec::ops_subcommand`; deps was missed.

**Why it matters**: a shim or stale `ops` earlier on PATH runs in place of the current binary (SEC-13), and `ops explain` shows `program: "ops"` for deps while every other builtin shows the resolved path.

**Origin**: discovered during TASK-2333 while fixing TASK-2326.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 deps registers via ExecCommandSpec::ops_subcommand (keeping its exclusive/read-only semantics deliberate) with a test pinning the resolved program

<!-- AC:END -->
