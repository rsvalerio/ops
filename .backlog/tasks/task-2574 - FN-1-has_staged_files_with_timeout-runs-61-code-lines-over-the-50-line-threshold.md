---
id: TASK-2574
title: 'FN-1: has_staged_files_with_timeout runs 61 code lines, over the 50-line threshold'
status: To Do
assignee: []
created_date: '2026-10-10 15:43'
updated_date: '2026-10-10 21:12'
labels:
  - code-review-rust
  - fn
dependencies: []
parent_task_id: 'TASK-2624'
modified_files:
  - extensions/hook-common/src/git_state.rs
priority: low
ordinal: 1000
dedup_key: 'FN-1:extensions/hook-common/src/git_state.rs:has_staged_files_with_timeout'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/hook-common/src/git_state.rs:162-241`

**What**: `has_staged_files_with_timeout` spans 76 lines with docs, 61 non-comment code lines — over the FN-1 threshold of 50. The body mixes four phases at different abstraction levels: subprocess spawn with typed error mapping (171-182), stderr drain-thread setup over a channel (188-201), the bounded wait/timeout/kill sequence (206-222), and exit-code classification (229-240). `read_stderr_bounded` is already a well-extracted helper, which makes the remaining inlining inconsistent.

**Why it matters**: FN-1 — each function should operate at a single abstraction level; a pre-commit critical-path primitive is frequently-read code where reviewers benefit from the drain setup and the exit-code classification being named, testable helpers.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 has_staged_files_with_timeout is at most 50 non-comment code lines, or carries a comment justifying the exception
- [ ] #2 Extracted helpers (e.g. spawn-drain-thread, classify-diff-exit-status) keep the existing behaviour and typed errors
- [ ] #3 cargo test -p ops-hook-common passes
<!-- AC:END -->
