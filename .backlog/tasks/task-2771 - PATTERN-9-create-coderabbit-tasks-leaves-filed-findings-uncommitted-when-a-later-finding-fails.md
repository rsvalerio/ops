---
id: TASK-2771
title: 'PATTERN-9: create-coderabbit-tasks leaves filed findings uncommitted when a later finding fails'
status: Done
assignee: []
created_date: '2026-11-18 18:43'
updated_date: '2026-11-18 23:05'
labels:
  - code-review-rust
  - PATTERN
dependencies: []
parent_task_id: 'TASK-2860'
modified_files:
  - extensions/create-coderabbit-tasks/src/lib.rs
priority: medium
ordinal: 1000
dedup_key: 'PATTERN-9:extensions/create-coderabbit-tasks/src/lib.rs:TargetRun::review'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/create-coderabbit-tasks/src/lib.rs:284-304` (`TargetRun::review`)

**What**: `TargetRun::review` files each finding with `create_task` inside the loop and only commits the new task files (`run_commit`) after the loop finishes. Any `?` inside the loop aborts the target after some tasks were already written to `.backlog/tasks`: `finding::repo_relative_file` rejecting one `fileName` (an absolute path outside the worktree, a `..`, a control character), or a later `create_task` I/O error. The already-written task files are left uncommitted in the main checkout, and the error does not name them. A re-run does not repair this: `--unless-exists` reports each of them as `Created::Exists`, and only `Created::New` ids go into `CommitOptions::task_ids`, so they are never committed by the tool.

A single malformed finding from CodeRabbit also discards every finding after it in the same review.

**Why it matters**: The commit is the durable half of the operation. The worktree cleanup is a guard (correct), but the commit has no equivalent: an early exit skips it and nothing reconciles it later. The user is left with stray untracked task files and a misleading "N new" count on the next run.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A finding whose fileName is rejected is skipped with a warning (or collected and reported after the loop) instead of aborting the target's remaining findings
- [x] #2 Task files created before any failure in TargetRun::review are still committed, or the error message names their ids and the ops backlog commit command that commits them
- [x] #3 A test covers a review whose stream contains one invalid fileName between two valid findings

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.24 (code-review-rust run, 2026-11-18).

Fixed in wave TASK-2860: filing extracted to file_findings; a refused fileName is skipped with a 'skipped:' line and counted; task files filed before a failure are committed anyway, and a failed commit names the ids and the ops backlog commit command. Test: an_invalid_file_name_skips_only_its_own_finding.

<!-- SECTION:NOTES:END -->
