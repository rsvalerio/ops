---
id: TASK-0185
title: 'ARCH-9: sql/ingest.rs uses glob import of validation module'
status: To Do
assignee: []
created_date: '2026-04-22 21:25'
updated_date: '2026-04-23 06:45'
labels:
  - rust-code-review
  - ARCH
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/ingest.rs:7`

**What**: ingest.rs uses glob import from super::validation, pulling every public item into scope. Any new validation helper leaks transitively. Obscures which symbols each call site actually depends on.

**Why it matters**: ARCH-9 — minimal public surface and explicit dependencies. Prefer explicit use items.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Glob imports are replaced with explicit item lists in sql/ingest.rs
- [ ] #2 sql/mod.rs re-export list is reviewed and narrowed to the intended public API
<!-- AC:END -->
