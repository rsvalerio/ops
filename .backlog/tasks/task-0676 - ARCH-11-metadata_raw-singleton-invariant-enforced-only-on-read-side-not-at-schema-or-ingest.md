---
id: TASK-0676
title: >-
  ARCH-11: metadata_raw singleton invariant enforced only on read side, not at
  schema or ingest
status: Done
assignee:
  - TASK-0738
created_date: '2026-04-30 05:14'
updated_date: '2026-04-30 18:35'
labels:
  - code-review-rust
  - architecture
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/ingestor.rs:65` and `extensions-rust/metadata/src/lib.rs:185`

**What**: `SELECT workspace_root FROM metadata_raw LIMIT 1` is read after a separate count, but `lib.rs:185` enforces "exactly one row" only on the read side, while the ingestor doesn't enforce the same invariant before the read.

**Why it matters**: Two transactions, no row-locking; if a concurrent ingestor inserts another row, the LIMIT 1 returns an arbitrary one.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either constrain metadata_raw to a singleton at schema level (e.g. id INTEGER PRIMARY KEY CHECK(id=0)) or read deterministically (ORDER BY + assertion)
<!-- AC:END -->
