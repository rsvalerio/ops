---
id: TASK-0614
title: >-
  READ-5: MetadataIngestor::load workspace_root extract via query_row<String>
  errors with low-context message
status: Done
assignee:
  - TASK-0644
created_date: '2026-04-29 05:20'
updated_date: '2026-04-29 17:07'
labels:
  - code-review-rust
  - READ
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/ingestor.rs:47`

**What**: query_row("SELECT workspace_root FROM metadata_raw LIMIT 1", ...) typed as String. If DuckDB infers workspace_root as null or non-VARCHAR (cargo metadata edge case), query errors with low-context "metadata_raw workspace_root extract" message and entire ingest fails.

**Why it matters**: Reliability — ingest failure cascades into about/coverage stack. tracing::warn with the underlying value would help diagnosis.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Error includes full row JSON or at minimum the column type observed
<!-- AC:END -->
