---
id: TASK-0525
title: 'READ-5: DuckDb::open_readonly does not create parent directory while open does'
status: Done
assignee:
  - TASK-0534
created_date: '2026-04-28 06:52'
updated_date: '2026-04-28 19:02'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/connection.rs:47`

**What**: DuckDb::open calls create_dir_all(parent) before opening; open_readonly skips this. A read-only open against a non-existent path fails with a less helpful duckdb-level error than the parent-mkdir failure would produce.

**Why it matters**: API asymmetry: callers that toggle between RW and RO modes get different error shapes for what is fundamentally the same problem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Drop create_dir_all from open (more honest) or document why open_readonly should also skip dir creation
- [ ] #2 Test for the asymmetric error shape
<!-- AC:END -->
