---
id: TASK-0516
title: >-
  ERR-1: warm_generic_providers swallows non-NotFound errors for duckdb/tokei
  warmup
status: Done
assignee:
  - TASK-0534
created_date: '2026-04-28 06:51'
updated_date: '2026-04-28 18:58'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/lib.rs:106`

**What**: `let _ = ctx.get_or_provide("duckdb", data_registry); let _ = ctx.get_or_provide("tokei", ...);` silently discards every error including non-NotFound failures. The coverage branch a few lines down does match-and-warn correctly, so the inconsistency is intra-function.

**Why it matters**: A duckdb open failure (e.g. permissions / disk full) becomes invisible — the about page just shows zeros. The pattern was applied carefully for coverage but not the others.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Apply the same Ok/NotFound/warn pattern to duckdb and tokei warmup
- [ ] #2 Tests cover the warn path
<!-- AC:END -->
