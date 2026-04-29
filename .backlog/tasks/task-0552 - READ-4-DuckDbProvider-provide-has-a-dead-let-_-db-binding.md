---
id: TASK-0552
title: 'READ-4: DuckDbProvider::provide has a dead let _ = db binding'
status: Done
assignee:
  - TASK-0642
created_date: '2026-04-29 05:02'
updated_date: '2026-04-29 12:51'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/lib.rs:110`

**What**: DuckDbProvider::provide does `if let Some(ref db) = ctx.db { let _ = db; return Ok(Value::Null); }`. The db binding is never read; let _ = db; exists only to silence a warning. The check could simply be if ctx.db.is_some().

**Why it matters**: Reads as if db were going to be used; future maintainers will look for a missing call. Minor but appears in a security-sensitive provider entry-point.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Replaced with if ctx.db.is_some() { return Ok(serde_json::Value::Null); }
- [x] #2 Behaviour unchanged
<!-- AC:END -->
