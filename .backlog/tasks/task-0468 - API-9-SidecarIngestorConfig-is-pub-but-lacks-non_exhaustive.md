---
id: TASK-0468
title: 'API-9: SidecarIngestorConfig is pub but lacks #[non_exhaustive]'
status: To Do
assignee:
  - TASK-0534
created_date: '2026-04-28 05:46'
updated_date: '2026-04-28 07:14'
labels:
  - code-review-rust
  - API
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/ingestor.rs:37`

**What**: SidecarIngestorConfig is a public struct (re-exported at crate root) with three pub fields. Adding a fourth field (e.g. view_filename or per-source extra_opts) is a breaking change for downstream extensions constructing it via struct-init.

**Why it matters**: This struct is the documented extension point for new sidecar-based ingestors. Marking #[non_exhaustive] and adding a new(...) constructor lets the workspace add fields without bumping every downstream extension.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 SidecarIngestorConfig is annotated #[non_exhaustive] and gains a pub fn new(name, json_filename, count_table) -> Self constructor
- [ ] #2 All in-tree call sites (tokei, coverage) use ::new instead of struct-init
<!-- AC:END -->
