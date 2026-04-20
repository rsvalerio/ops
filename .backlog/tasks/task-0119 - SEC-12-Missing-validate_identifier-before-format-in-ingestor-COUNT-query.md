---
id: TASK-0119
title: 'SEC-12: Missing validate_identifier before format! in ingestor COUNT query'
status: Done
assignee: []
created_date: '2026-04-19 18:51'
updated_date: '2026-04-19 20:31'
labels:
  - rust-code-review
  - security
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/ingestor.rs:74`

**What**: `format!("SELECT COUNT(*) FROM {}", self.count_table)` interpolates `self.count_table` directly into SQL without calling `validate_identifier()`. Sister code at `extensions/duckdb/src/sql/ingest.rs:55` calls `validate_identifier(table_name)?` before a nearly identical `format!("SELECT COUNT(*) FROM \"{table_name}\"")`. Though `count_table` is currently typed `&'static str` (compile-time-constant), the inconsistency makes the safety property non-local and easy to regress if the field type is widened.

**Why it matters**: Consistent validation at all SQL interpolation sites is the defense-in-depth invariant that prevents SQL injection if a code change later lets non-constant input reach this path. Reviewers auditing duckdb sinks currently have to prove each site safe individually rather than trusting a uniform pattern.

<!-- scan confidence: verified candidate -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either validate_identifier is called on count_table before the format!, or a doc comment on IngestorSpec.count_table documents the `&\'static str` invariant and cross-references the assumption
- [ ] #2 No regression of behavior for the normal path (inputs from existing call sites unchanged)
<!-- AC:END -->
