---
id: TASK-2386
title: 'FN-1: extract_required_version is ~100 lines mixing three jobs'
status: Done
assignee: []
created_date: '2026-10-04 14:13'
updated_date: '2026-10-04 16:14'
labels:
  - code-review-rust
  - function-structure
dependencies: []
parent_task_id: 'TASK-2424'
modified_files:
  - extensions-terraform/about/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'FN-1:extensions-terraform/about/src/lib.rs:extract_required_version'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/about/src/lib.rs:224-326`

**What**: `extract_required_version` spans 103 lines (about 55 of code). It reports `StripEof` outcomes, drives the per-line scan loop with a first-wins `found` accumulator, and performs the end-of-file structural refusals (open heredoc, non-empty stack) in one body, each with its own `tracing::warn!` plus `return None`. This is not a state-machine exception: the state machine already lives in `scan_line`.

**Why it matters**: The function is the choke point for every refusal reason; adding another refusal grows an already over-limit function. Its doc describes three separable stages, but this orchestrator is the part that is not.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 StripEof reporting and the end-of-file checks are extracted into named helpers, with one place emitting the warn
- [x] #2 extract_required_version is at most 50 lines and all existing extract_required_version_* tests pass unchanged

<!-- AC:END -->
