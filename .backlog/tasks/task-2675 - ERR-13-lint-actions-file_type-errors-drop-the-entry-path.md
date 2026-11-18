---
id: TASK-2675
title: 'ERR-13: lint-actions file_type errors drop the entry path'
status: Done
assignee: []
created_date: '2026-11-18 18:26'
updated_date: '2026-11-18 22:07'
labels:
  - code-review-rust
  - ERR
dependencies: []
parent_task_id: 'TASK-2843'
modified_files:
  - crates/cli/src/lint_actions_cmd.rs
priority: low
ordinal: 1000
dedup_key: 'ERR-13:crates/cli/src/lint_actions_cmd.rs:workflow_files'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/lint_actions_cmd.rs:106` (`workflow_files`), `crates/cli/src/lint_actions_cmd.rs:133` (`action_files`)

**What**: `entry.file_type()?` propagates a bare `io::Error` with no path, while every neighbouring fs call in the module attaches `with_context(|| format!("reading {}", ...))`.

**Why it matters**: A failing `file_type` (e.g. a racing delete or a permission error on a workflow entry) surfaces as "No such file or directory" with no indication of which workflow or action directory entry failed.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Both entry.file_type() calls attach the entry path as context

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.02 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
