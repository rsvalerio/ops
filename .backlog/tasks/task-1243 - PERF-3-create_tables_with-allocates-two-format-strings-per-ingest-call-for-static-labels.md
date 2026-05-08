---
id: TASK-1243
title: >-
  PERF-3: create_tables_with allocates two format strings per ingest call for
  static labels
status: To Do
assignee:
  - TASK-1263
created_date: '2026-05-08 13:00'
updated_date: '2026-05-08 13:18'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/ingestor.rs:216-219`

**What**: `create_tables_with` calls `format!("{} create", self.name)` / `format!("{} view", self.name)` eagerly on every successful load — `self.name` is &'static str, the suffixes are literals, and the resulting strings are only consumed inside the `map_err` closure on the failure path.

**Why it matters**: Two unconditional heap allocations on every ingest cycle (tokei/coverage/metadata) for an error label that the success path discards.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Move the format! into the map_err closure so the happy path allocates nothing
- [ ] #2 Apply the same pattern to count_records_with's analogous label
- [ ] #3 Microbench / dhat-style alloc test pinning zero allocations on success
<!-- AC:END -->
