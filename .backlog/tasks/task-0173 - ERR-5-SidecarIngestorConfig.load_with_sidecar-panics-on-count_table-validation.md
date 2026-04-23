---
id: TASK-0173
title: >-
  ERR-5: SidecarIngestorConfig.load_with_sidecar panics on count_table
  validation
status: To Do
assignee: []
created_date: '2026-04-22 21:25'
updated_date: '2026-04-23 06:45'
labels:
  - rust-code-review
  - ERR
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/ingestor.rs:76-81`

**What**: load_with_sidecar calls validate_identifier and panics via unwrap_or_else on failure. count_table is &static str so all current inputs are checked at the call site, but a panic in a library load path is a poor failure mode — aborts the whole process instead of returning a DbError.

**Why it matters**: ERR-5 — production panic where a Result is available and idiomatic. Defense-in-depth that becomes offensive. Propagate via DbError::query_failed or a new DbError::InvalidIdentifier variant. Related to TASK-0132 but the finding here is the caller choosing to panic.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 load_with_sidecar returns a DbError instead of panicking on invalid count_table
- [ ] #2 test asserts the error path with a deliberately invalid identifier
<!-- AC:END -->
