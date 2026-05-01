---
id: TASK-0665
title: >-
  ERR-7: workspace.rs path log fields use Display, allowing log injection via
  path
status: Done
assignee:
  - TASK-0743
created_date: '2026-04-30 05:13'
updated_date: '2026-04-30 20:26'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/workspace.rs:51, 58-63, 78-95`

**What**: `try_read_manifest` and `resolve_member_globs` log `manifest = %path.display()` (Display formatting). Manifest paths containing newlines or ANSI escapes (legal on Unix) can forge log lines, while sibling DuckDB error contexts (extensions/duckdb/src/sql/ingest.rs:55-59) deliberately use Debug to defang this.

**Why it matters**: Same defense applied for SQL identifier log-injection should apply to user-supplied paths flowing into about-stack logs. Cheap fix; closes a parity gap rather than a live exploit.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Switch path = %path.display() to path = ?path.display() (or Debug-format the value) at this call site and the matching arms in resolve_member_globs
- [x] #2 Add a regression test that a path containing newlines/ANSI escapes is escaped in the rendered log line
<!-- AC:END -->
