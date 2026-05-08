---
id: TASK-1223
title: >-
  PERF-3: atomic_write tmp_name composes via intermediate format!() String per
  write
status: To Do
assignee:
  - TASK-1263
created_date: '2026-05-08 12:57'
updated_date: '2026-05-08 13:18'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/edit.rs:128-132`

**What**: `tmp_name` builds an OsString with capacity hint, then `tmp_name.push(format!(".tmp.{pid}.{counter}.{nanos}"))` allocates a fresh String via format! before pushing into the OsString. Two allocations per atomic write where one would do.

**Why it matters**: `atomic_write` is on every .ops.toml write path. The format! allocation duplicates the OsString growth that already exists, doubling bookkeeping.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Push pid/counter/nanos as separate appends via itoa or write! to a reused buffer
- [ ] #2 End-state OsString unchanged
- [ ] #3 Existing crash-safety / tmp-name uniqueness tests pass
<!-- AC:END -->
