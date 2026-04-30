---
id: TASK-0666
title: 'ERR-1: io_err mislabels arbitrary errors as DbError::Io'
status: Done
assignee:
  - TASK-0737
created_date: '2026-04-30 05:13'
updated_date: '2026-04-30 17:52'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/ingest.rs:106-108`

**What**: `io_err` coerces *any* `Into<Box<dyn Error+Send+Sync>>` into `DbError::Io(std::io::Error::other(e))`, masquerading non-IO errors (`serde_json::Error`, `tokei` failures, etc.) as IO errors. The only call sites that need this are in `tokei::ingestor` and `collect_sidecar` for `serde_json::to_vec_pretty`.

**Why it matters**: Operators reading `DbError::Io(...)` reasonably assume an IO problem; a JSON serialisation failure presenting as IO sends them down the wrong diagnostic path. `DbError` already has a `Serialization(serde_json::Error)` variant — wrapping serde errors as IO loses that classification.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace io_err(serde_json::to_vec_pretty(...).err()) in collect_sidecar (and equivalent tokei sites) with DbError::Serialization / a dedicated variant
- [ ] #2 Either delete io_err or rename it (e.g. external_io_err) and document that callers must have already classified the error as IO
<!-- AC:END -->
