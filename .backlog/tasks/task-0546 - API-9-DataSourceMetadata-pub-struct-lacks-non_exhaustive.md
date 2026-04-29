---
id: TASK-0546
title: 'API-9: DataSourceMetadata pub struct lacks #[non_exhaustive]'
status: Triage
assignee: []
created_date: '2026-04-29 05:01'
labels:
  - code-review-rust
  - api-design
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/schema.rs:50`

**What**: pub struct DataSourceMetadata (with a lifetime parameter) is re-exported (ops_duckdb::DataSourceMetadata) and used by extension ingestors, but exposes all fields as pub without #[non_exhaustive] and has no constructor — every call site uses struct-literal init.

**Why it matters**: Adding a new tracking field (e.g. ingest duration, schema version) is a breaking change for every ingestor.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Struct is annotated #[non_exhaustive] with a pub fn new(...) (or builder) provided
- [ ] #2 Internal callers are migrated to the constructor; struct-literal use from outside the crate is rejected by the compiler
<!-- AC:END -->
