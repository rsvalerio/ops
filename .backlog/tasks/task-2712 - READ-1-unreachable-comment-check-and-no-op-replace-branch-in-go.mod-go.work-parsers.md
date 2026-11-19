---
id: TASK-2712
title: 'READ-1: unreachable comment check and no-op replace branch in go.mod/go.work parsers'
status: To Do
assignee: []
created_date: '2026-11-18 18:32'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - READ
dependencies: []
parent_task_id: 'TASK-2846'
modified_files:
  - extensions-go/about/src/go_work.rs
  - extensions-go/about/src/go_mod.rs
priority: low
ordinal: 1000
dedup_key: 'review-2667.07-4'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-go/about/src/go_work.rs:56`, `extensions-go/about/src/go_mod.rs:102`

**What**:
- go_work.rs:56-58 — `if line.starts_with("//") { continue; }` inside the use block is unreachable: `line` is already `strip_line_comment(raw).trim()` (go_work.rs:42), which reduces any comment-only line to `""`, and empty lines `continue` at line 43.
- go_mod.rs:102-105 — `else if strip_verb(line, "replace").is_some() { /* Skipped */ }` is an empty branch: removing it changes nothing, since the if-chain has no trailing `else`.

**Why it matters**: Dead guards suggest an input shape the parser supposedly handles there, so readers (and future editors of the comment policy) reason about a path that cannot run. The no-op branch reads as if replace lines need special handling at the top level when they do not.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The unreachable starts_with("//") check in go_work::parse_use_dirs is removed
- [ ] #2 The empty replace branch in go_mod::parse is removed (a short comment may note that top-level replace lines are intentionally ignored)
- [ ] #3 All existing parser tests pass unchanged
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.07 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
