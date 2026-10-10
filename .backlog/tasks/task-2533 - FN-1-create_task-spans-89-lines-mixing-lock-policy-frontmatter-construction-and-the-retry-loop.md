---
id: TASK-2533
title: 'FN-1: create_task spans ~89 lines mixing lock policy, frontmatter construction, and the retry loop'
status: Done
assignee: []
created_date: '2026-10-10 15:34'
updated_date: '2026-10-10 21:33'
labels:
  - code-review-rust
  - functions
dependencies: []
parent_task_id: 'TASK-2613'
modified_files:
  - crates/backlog/src/cmd/create.rs
priority: medium
ordinal: 1000
dedup_key: 'FN-1:crates/backlog/src/cmd/create.rs:create_task'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/backlog/src/cmd/create.rs:79` (create_task, ~89 raw lines to line 168)

**What**: create_task exceeds the 50-line FN-1 threshold. One body holds three concerns: the unless-exists lock/short-circuit policy (lines 84-101), a ~35-line Frontmatter literal construction with per-field comments (lines 115-133), and the create_new contention-retry loop (lines 107-167).

**Why it matters**: The retry invariant (CREATE_ATTEMPTS, re-scan on AlreadyExists) is the concurrency-critical part and reads hardest wrapped around the field-by-field struct build. Extracting the frontmatter/body construction (`frontmatter_for(opts, cfg, id, stamp, key)`) and keeping the loop minimal separates the policy from the payload.

<!-- scan confidence: raw-line measurement including comments -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 create_task's frontmatter/body construction is a named helper; the function stays under ~50 lines
- [x] #2 Lock scope, retry-on-AlreadyExists, and unless-exists short-circuit behaviour is unchanged (existing tests pass)

<!-- AC:END -->
