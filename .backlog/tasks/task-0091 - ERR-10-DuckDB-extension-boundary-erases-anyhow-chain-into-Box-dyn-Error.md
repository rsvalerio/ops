---
id: TASK-0091
title: 'ERR-10: DuckDB extension boundary erases anyhow chain into Box<dyn Error>'
status: To Do
assignee: []
created_date: '2026-04-17 11:33'
updated_date: '2026-04-17 12:07'
labels:
  - rust-codereview
  - err
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/lib.rs:43`

**What**: try_provide_from_db returns Result<Value, DataProviderError> via .map_err(Into::into), erasing the original anyhow::Error chain.

**Why it matters**: Callers lose .context() chains and typed match arms; ERR-10 prefers typed errors at crate boundaries.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Add a dedicated variant to DataProviderError for DuckDB/ingestor failures
- [ ] #2 Preserve anyhow::Error chain via source() by wrapping in a thiserror variant
<!-- AC:END -->
