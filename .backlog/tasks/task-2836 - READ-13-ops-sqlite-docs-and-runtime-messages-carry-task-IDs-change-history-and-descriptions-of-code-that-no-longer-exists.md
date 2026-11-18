---
id: TASK-2836
title: 'READ-13: ops-sqlite docs and runtime messages carry task IDs, change history and descriptions of code that no longer exists'
status: Done
assignee: []
created_date: '2026-11-18 18:51'
updated_date: '2026-11-18 21:47'
labels:
  - code-review-rust
  - READ
dependencies: []
parent_task_id: 'TASK-2853'
modified_files:
  - extensions/sqlite/src/sql/query/helpers.rs
  - extensions/sqlite/src/sql/ingest/orchestrator.rs
  - extensions/sqlite/src/sql/query/deps.rs
  - extensions/sqlite/src/sql/ingest/mod.rs
  - extensions/sqlite/src/sql/ingest/sidecar.rs
  - extensions/sqlite/src/sql/validation.rs
  - extensions/sqlite/src/connection.rs
  - extensions/sqlite/src/lib.rs
  - extensions/sqlite/src/error.rs
  - extensions/sqlite/src/ingestor.rs
  - extensions/sqlite/Cargo.toml
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions/sqlite/src:crate-docs'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/sqlite/src/sql/query/helpers.rs:40`, `extensions/sqlite/src/sql/ingest/orchestrator.rs:33`, `extensions/sqlite/src/sql/query/deps.rs:132`, `extensions/sqlite/src/sql/ingest/mod.rs:3-5`, `extensions/sqlite/src/sql/ingest/sidecar.rs:42-47`, `extensions/sqlite/src/sql/validation.rs:38-41`, `extensions/sqlite/src/connection.rs:57-70,165`, `extensions/sqlite/Cargo.toml:26-28`, plus `// READ-10 / TASK-1873` style comments in `lib.rs:22,91,105`, `error.rs:6`, `connection.rs:73`, `ingestor.rs:4`

**What**:
- Task and rule IDs leak into rendered or emitted text. The rustdoc of `QueryTableName` (passed through `sql_ident_newtype!`) begins "READ-1 / TASK-1624: …". The `debug_assert!` message in `ReentryGuard::new` begins "CONC-2 / TASK-1143: …". The `tracing::debug!` message in `query_crate_dep_counts` begins "ERR-2 / TASK-1253: …". The workspace-wide cleanup (92a8dc3b, "drop task and review-rule ids from doc comments") missed these because they are macro or string literals.
- Docs narrate the journey: `ingest/mod.rs` "Split the previous 1500+ line module … downstream callers see no churn"; `ingest_locks` "instead of leaking … in a process-global `OnceLock` … the signature change makes …"; the `lib.rs`/`error.rs`/`connection.rs` comments explaining which `#[allow]`s "used to" exist.
- Docs describe code that is gone or behaviour that does not hold: `MAX_SIDECAR_BYTES` guards "the unsafe `from_encoded_bytes_unchecked` boundary", but the reader is now unsafe-free. `SqlError::PathTraversalNotAllowed` says "via `..` or an absolute prefix", but `validate_no_traversal` rejects only `..`. `Sqlite::path` claims a "resolved absolute path", which is not true for `:memory:` or a relative root. The `libc` comment in Cargo.toml says it decides whether "a sticky shared-writable ingest staging parent is owned by a principal we trust", but the parent is no longer policed and `libc` serves the leaf owner check and `*at` syscalls.

**Why it matters**: Readers and operators see tracker IDs that mean nothing to them. Stale claims about safety boundaries and validation scope look authoritative and mislead the next change.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 No rule or task ID appears in a rustdoc string, panic/assert message or log message in ops-sqlite production code
- [x] #2 Module and item docs describe the current behaviour only; history narration and 'used to' comments are removed
- [x] #3 MAX_SIDECAR_BYTES, SqlError::PathTraversalNotAllowed, Sqlite::path and the Cargo.toml libc comment describe what the code does today

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.30 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
