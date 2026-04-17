---
id: TASK-0062
title: 'FN-1: run_commands mixes plan merging, runtime and display setup in 71 lines'
status: To Do
assignee: []
created_date: '2026-04-17 11:30'
updated_date: '2026-04-17 12:07'
labels:
  - rust-codereview
  - fn
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/run_cmd.rs:42`

**What**: `run_commands` handles runner build, dry-run branch, leaf merging with parallel/fail_fast detection, display setup, runtime creation, and result post-processing in one 71-line body.

**Why it matters**: Several distinct concerns are interleaved, making it harder to unit test the plan-merging logic in isolation.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extract a pure plan-merge helper returning (leaf_ids, any_parallel, fail_fast)
- [ ] #2 Extract the tokio runtime block into a helper taking a closure selecting parallel vs sequential
- [ ] #3 Body under 50 lines
<!-- AC:END -->
