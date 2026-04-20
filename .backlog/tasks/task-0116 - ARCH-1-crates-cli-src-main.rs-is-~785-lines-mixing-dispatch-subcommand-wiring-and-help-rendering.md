---
id: TASK-0116
title: >-
  ARCH-1: crates/cli/src/main.rs is ~785 lines mixing dispatch, subcommand
  wiring, and help rendering
status: Done
assignee: []
created_date: '2026-04-19 18:41'
updated_date: '2026-04-19 19:25'
labels:
  - rust-code-review
  - architecture
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/main.rs:1-785`

**What**: Top-level CLI entrypoint contains subcommand dispatch, help/categorization logic, and per-command setup in a single file.

**Why it matters**: A ~785-line main.rs couples orchestration to presentation and complicates unit testing of dispatch rules; a thin main.rs delegating to dedicated command modules is easier to evolve.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 main.rs is reduced to under ~200 lines of dispatch; category/help rendering moved to dedicated module
- [x] #2 cargo test still passes and  output is unchanged
<!-- AC:END -->
