---
id: TASK-0593
title: >-
  SEC-12: tokei_languages_view_sql hard-codes table identifiers without
  quoted_ident wrapper
status: Triage
assignee: []
created_date: '2026-04-29 05:18'
labels:
  - code-review-rust
  - SEC
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/tokei/src/views.rs:16`

**What**: tokei_languages_view_sql returns static SQL with "tokei_languages" and "tokei_files" interpolated as bare identifiers. Static today, but the function is pub and the surrounding module ethos (TableName/ColumnName newtypes, quoted_ident defense-in-depth) is to route every interpolated identifier through quoted_ident. This view bypasses that policy.

**Why it matters**: SEC-12 (defense-in-depth). A single un-quoted call site is the kind of inconsistency that makes a future SEC-12 regression impossible to spot at the diff level.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 tokei_languages_view_sql constructs identifiers via quoted_ident or TableName::new
- [ ] #2 Output SQL contains double-quoted identifiers
- [ ] #3 Test pins parity with sister tokei_files_create_sql
<!-- AC:END -->
