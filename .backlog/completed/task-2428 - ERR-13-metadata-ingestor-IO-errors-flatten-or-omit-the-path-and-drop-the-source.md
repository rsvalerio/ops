---
id: TASK-2428
title: 'ERR-13: metadata ingestor IO errors flatten or omit the path and drop the source'
status: Done
assignee: []
created_date: '2026-10-04 15:28'
updated_date: '2026-10-10 14:48'
labels:
  - code-review-rust
  - ERR
dependencies: []
parent_task_id: 'TASK-2441'
modified_files:
  - extensions-rust/metadata/src/ingestor.rs
  - extensions/sqlite/src/error.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/ingestor.rs` (`io_at` ~L219, `read_staged_payload` ~L182)

**What**: `io_at` rebuilds the `io::Error` with `format!("{op} {path}: {e}")`, which discards the original error as source (the cause survives only as flattened text). `read_staged_payload` maps a `read_to_string` failure with a bare `DbError::Io`, naming no path. ops-sqlite now has a crate-private `io_context` helper (context as message, original error as source) and `IngestDir::entry_error`, neither reachable from this crate.

**Why it matters**: After TASK-2406/TASK-2407 every other `DbError::Io` in the ingest pipeline carries the path as message and the OS error as source; these two sites are the remaining exceptions, so chain walkers and typed downcasts see a different shape here.

**Origin**: discovered during TASK-2416 while fixing TASK-2407.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 io_at keeps the original io::Error reachable through Error::source()
- [x] #2 The staged-payload read failure names the staged entry

<!-- AC:END -->
