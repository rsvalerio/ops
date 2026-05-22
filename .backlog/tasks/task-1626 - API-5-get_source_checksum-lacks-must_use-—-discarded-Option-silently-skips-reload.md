---
id: TASK-1626
title: >-
  API-5: get_source_checksum lacks #[must_use] — discarded Option silently skips
  reload
status: Done
assignee:
  - TASK-1640
created_date: '2026-05-22 07:12'
updated_date: '2026-05-22 13:43'
labels:
  - code-review-rust
  - api
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/schema.rs:30`

**What**: `pub fn get_source_checksum(...) -> DbResult<Option<String>>` is the function callers consult to decide whether a data source is already current (`Some(checksum)`) or has never been ingested (`None`). It lacks `#[must_use]`. A caller that accidentally drops the return value treats "no record" identically to "already current", masking ingest failures.

**Why it matters**: Mirrors the pattern called out by the project's own task-1593 (public report-returning functions need `#[must_use]`). The signal-carrying `Option<String>` should be hard to ignore by accident.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 #[must_use] attribute (with a short reason string) added to get_source_checksum
- [ ] #2 Workspace builds clean with no new warnings
<!-- AC:END -->
