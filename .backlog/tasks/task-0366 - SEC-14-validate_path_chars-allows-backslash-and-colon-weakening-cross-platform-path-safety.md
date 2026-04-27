---
id: TASK-0366
title: >-
  SEC-14: validate_path_chars allows backslash and colon, weakening
  cross-platform path safety
status: Done
assignee:
  - TASK-0419
created_date: '2026-04-26 09:36'
updated_date: '2026-04-27 10:54'
labels:
  - code-review-rust
  - security
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/validation.rs:130`

**What**: validate_path_chars allows \\ and : so Windows paths pass. On Unix, : is the PATH-list separator and \\ has no path meaning. Combined prepare_path_for_sql SQL-escapes \\ but not :.

**Why it matters**: Defense-in-depth. A path like /tmp/foo:bar could carry shell-meaningful sequences into downstream contexts (logs, errors, future shell invocations).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Gate \\\\ and : behind cfg(windows) or document why they are accepted on every platform
- [x] #2 Tests cover Unix-only rejection of \\\\ and : in paths
<!-- AC:END -->
