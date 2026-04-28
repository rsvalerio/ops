---
id: TASK-0521
title: >-
  ERR-7: table_has_data formats identifier with Display while sister
  table_exists uses Debug
status: To Do
assignee:
  - TASK-0534
created_date: '2026-04-28 06:52'
updated_date: '2026-04-28 07:14'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/ingest.rs:78`

**What**: `format!("counting rows in {}", table_name)` interpolates with Display while the immediately preceding table_exists uses Debug specifically to defang control-character/log-injection (see ERR-7 comment in same file).

**Why it matters**: Two error-context sites in one function with divergent escaping policies — the safer one was added explicitly; the second was missed. Same risk surface.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Switch to {table_name:?} to match table_exists
- [ ] #2 Test mirroring table_exists_error_message_sanitizes_control_chars for table_has_data
<!-- AC:END -->
