---
id: TASK-0132
title: 'SEC-12: validate_identifier panics instead of returning Result'
status: To Do
assignee: []
created_date: '2026-04-22 21:16'
updated_date: '2026-04-23 06:45'
labels:
  - rust-code-review
  - sec
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/ingestor.rs:76-81` (around `format!("SELECT COUNT(*) FROM \"{}\"", self.count_table)`)

**What**: Defense-in-depth identifier validation is implemented as an assertion that panics on failure. Today `count_table` is `&'static str`, but if the bound is ever widened to `String`, the only line of defense becomes a panic in library code.

**Why it matters**: Panics in libraries crash the whole process. A returned Result is both safer and more testable.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 validate_identifier returns Result<(), IdentifierError> rather than panicking
- [ ] #2 Callers propagate the error via ? with context
<!-- AC:END -->
