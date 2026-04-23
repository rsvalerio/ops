---
id: TASK-0202
title: >-
  READ-5: SidecarIngestorConfig::load_with_sidecar has undocumented multi-step
  side effects
status: To Do
assignee: []
created_date: '2026-04-22 21:26'
updated_date: '2026-04-23 06:45'
labels:
  - rust-code-review
  - READ
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/ingestor.rs:61-111`

**What**: load_with_sidecar creates table, creates view, reads sidecar file, computes checksum, upserts data_sources row, removes JSON and sidecar files. If any step fails mid-way the earlier steps are partially committed (table exists, sidecar on disk). No doc comment enumerates this lifecycle or failure semantics.

**Why it matters**: READ-4/READ-5/SEC-32 — make invariants and cleanup semantics explicit. A retrying caller may find a half-committed state. Document ordering, or use a scope guard to clean up on error.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 load_with_sidecar documents each side effect and its cleanup semantics on error
- [ ] #2 Failure after table creation cleans up sidecar/JSON or is explicitly called out as idempotent-on-retry
<!-- AC:END -->
