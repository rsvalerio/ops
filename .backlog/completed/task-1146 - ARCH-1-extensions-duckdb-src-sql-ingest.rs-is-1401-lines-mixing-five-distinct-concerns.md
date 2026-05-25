---
id: TASK-1146
title: >-
  ARCH-1: extensions/duckdb/src/sql/ingest.rs is 1401 lines mixing five distinct
  concerns
status: Done
assignee:
  - TASK-1264
created_date: '2026-05-08 07:42'
updated_date: '2026-05-09 12:14'
labels:
  - code-review-rust
  - ARCH
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/ingest.rs:1`

**What**: One module owns: (a) per-source CREATE TABLE SQL builder, (b) ingest-dir hardening with Unix 0o700 stamping (SEC-25/TASK-0787/1000), (c) sidecar I/O with size cap and Unix encoding (SEC-33/TASK-0951, UNSAFE-1/TASK-1104), (d) streaming SHA-256 (PERF-1), (e) the provide_via_ingestor orchestrator with per-table mutex / refresh / poison recovery (CONC-2/TASK-0728/0909/1073, ERR-5/TASK-0780), (f) DUP-032 test_create_sql_validation macro plus ~700 lines of #[cfg(test)].

**Why it matters**: Cross-cutting refactors require reading the entire file; reviewers triaging \"did the ingest mutex change?\" page through unrelated SEC-25 sidecar code.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Split into ingest/sidecar.rs, ingest/orchestrator.rs, ingest/sql.rs, ingest/dir.rs
- [x] #2 Keep sql/mod.rs's pub use surface identical so downstream callers see no churn
<!-- AC:END -->
