---
id: TASK-1120
title: >-
  ARCH-5: extensions-go/about go_mod and go_work modules have circular
  dependency
status: To Do
assignee:
  - TASK-1264
created_date: '2026-05-08 07:26'
updated_date: '2026-05-08 13:18'
labels:
  - code-review-rust
  - structure
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-go/about/src/go_mod.rs:44`, `extensions-go/about/src/go_work.rs:29,34,59`

**What**: `go_mod.rs::parse` calls `crate::go_work::is_block_opener`, while `go_work.rs::parse_use_dirs` and `go_work.rs::is_block_opener` call `crate::go_mod::strip_line_comment`. The two sibling modules import from each other in both directions.

**Why it matters**: ARCH-5 forbids circular dependencies between modules; they prevent extracting either module independently, force both to compile in lockstep, and signal that the shared lexical helpers (`strip_line_comment`, `is_block_opener`) belong in a third module that both can depend on. The current shape is a maintenance hazard and a refactoring blocker for future Go-syntax helpers.

**Suggested fix**: Extract `strip_line_comment` and `is_block_opener` into a shared `go_syntax.rs` (or similar) submodule and have both `go_mod` and `go_work` depend on it one-way.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Circular dependency between go_mod and go_work eliminated
- [ ] #2 Shared Go-syntax helpers live in a single module both depend on
<!-- AC:END -->
