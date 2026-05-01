---
id: TASK-0117
title: >-
  ARCH-1: crates/cli/src/run_cmd.rs is ~783 lines mixing plan merging, runtime
  setup, display wiring, and execution
status: Done
assignee: []
created_date: '2026-04-19 18:41'
updated_date: '2026-04-19 19:27'
labels:
  - rust-code-review
  - architecture
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/run_cmd.rs:1-783`

**What**: The `run` command module combines plan assembly, tokio runtime construction, display setup, and execution orchestration in one file.

**Why it matters**: Overlaps with existing findings (FN-1 task 0062 `run_commands`, DUP-1 task 0069 runtime scaffolding) but the module itself crossed the 500-line threshold since last review — pushes toward rising cognitive load and makes incremental refactors harder.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 run_cmd.rs split into submodules (e.g. plan_assembly, runtime, orchestration) each under ~400 lines
- [x] #2 cargo test, cargo clippy, and integration CLI tests all pass
<!-- AC:END -->
