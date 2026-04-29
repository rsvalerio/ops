---
id: TASK-0586
title: 'API-9: pub struct LoadResult lacks #[non_exhaustive]'
status: Done
assignee:
  - TASK-0636
created_date: '2026-04-29 05:17'
updated_date: '2026-04-29 06:15'
labels:
  - code-review-rust
  - API
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/ingestor.rs:16`

**What**: LoadResult is pub and re-exported from crate root (`pub use ingestor::LoadResult;` lib.rs:17). Two public fields, success constructor, no #[non_exhaustive]. Sister type SidecarIngestorConfig in same file already carries it (TASK-0468); LoadResult was missed.

**Why it matters**: API-9 — without non_exhaustive, evolving the load-result shape (e.g. adding bytes_loaded, duration_ms, skipped) breaks the public API. Project policy convergence (TASK-0234/0260/0436/0468).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 LoadResult annotated #[non_exhaustive]
- [ ] #2 All construction goes through LoadResult::success
- [ ] #3 Test pins out-of-crate struct-init forbidden
<!-- AC:END -->
