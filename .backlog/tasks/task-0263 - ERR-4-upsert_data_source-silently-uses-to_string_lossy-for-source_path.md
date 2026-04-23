---
id: TASK-0263
title: 'ERR-4: upsert_data_source silently uses to_string_lossy for source_path'
status: Done
assignee: []
created_date: '2026-04-23 06:36'
updated_date: '2026-04-23 09:18'
labels:
  - rust-code-review
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/schema.rs:59`

**What**: Non-UTF-8 path silently becomes replacement-char string before storage; stored row cannot be mapped back.

**Why it matters**: Silent corruption of stored metadata.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Fail fast on non-UTF-8 paths or persist as BLOB
- [x] #2 Test non-UTF-8 path rejected
<!-- AC:END -->
