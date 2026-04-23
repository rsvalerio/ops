---
id: TASK-0258
title: 'ERR-1: remove_workspace_sidecar discards all errors with let _ ='
status: To Do
assignee: []
created_date: '2026-04-23 06:36'
updated_date: '2026-04-23 06:46'
labels:
  - rust-code-review
  - error-handling
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/ingest.rs:126`

**What**: Remove failures (EACCES, stale handle) are fully swallowed with no log.

**Why it matters**: Accumulated sidecar files can mask that cleanup is broken.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Log via tracing::warn when remove fails (still best-effort)
- [ ] #2 Test asserting log emission for removal failure
<!-- AC:END -->
