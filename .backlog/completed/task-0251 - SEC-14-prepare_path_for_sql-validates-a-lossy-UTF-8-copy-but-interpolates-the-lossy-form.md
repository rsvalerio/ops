---
id: TASK-0251
title: >-
  SEC-14: prepare_path_for_sql validates a lossy UTF-8 copy but interpolates the
  lossy form
status: Done
assignee: []
created_date: '2026-04-23 06:35'
updated_date: '2026-04-23 09:02'
labels:
  - rust-code-review
  - security
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/validation.rs:84`

**What**: path.to_string_lossy() silently replaces invalid UTF-8 with U+FFFD prior to validate_path_chars; lossy string is then interpolated into SQL.

**Why it matters**: Undermines defense-in-depth model; behavior differs across platforms/paths.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Reject non-UTF-8 paths with SqlError::InvalidPathChar up front
- [x] #2 Unit test using OsString with invalid UTF-8 returns Err
<!-- AC:END -->
