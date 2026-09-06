---
id: TASK-1893
title: >-
  ERR-13: every filesystem failure in MetadataIngestor::collect surfaces as a
  bare 'IO error' with no path
status: Done
assignee:
  - TASK-1999
created_date: '2026-08-27 15:35'
updated_date: '2026-08-28 21:09'
labels:
  - code-review-rust
  - idioms-correctness
dependencies: []
modified_files:
  - extensions-rust/metadata/src/ingestor.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/ingestor.rs:20`, `:22`, `:39`

**What**: All three IO edges in `collect` map straight to `DbError::Io(io)`, whose Display is `"IO error: {0}"` — the underlying `std::io::Error` carries no path:

```rust
std::fs::create_dir_all(data_dir).map_err(DbError::Io)?;                       // :20 — which dir?
let output = run_cargo_metadata(&ctx.working_directory).map_err(|e| match e {
    ops_core::subprocess::RunError::Io(io) => DbError::Io(io),                 // :22 — which working dir?
    ...
})?;
ops_core::config::atomic_write(&path, &output.stdout).map_err(DbError::Io)?;   // :39 — which file?
```

The operator-visible result for an ENOSPC, EACCES or read-only-mount failure is `IO error: Permission denied (os error 13)` with nothing naming `<db-dir>/ingest/metadata.json`, the ingest dir, or the working directory. The cross-crate contributor is `ops_core::config::atomic_write` (`crates/core/src/config/edit.rs:194`), which returns a plain `std::io::Result<()>` and does not attach the destination path — but the fix belongs here regardless, since `collect` is the layer that knows all three paths.

`cleanup_staged_file` (:132) already does this right (`path = %path.display()` in its warn), which makes the omission on the failing paths more conspicuous, not less.

**Why it matters**: ERR-13. This is ingest of a file under a DuckDB data directory the operator never chose and cannot guess from the message. In a CI log the bare errno is unactionable: three different filesystem operations on three different paths all render identically, so the first diagnostic step is always "read the source to find out which one".
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 create_dir_all, run_cargo_metadata's Io arm, and atomic_write each produce an error naming the path (or working directory) they operated on
- [x] #2 The three failures are distinguishable from one another in the rendered message without reading the source
- [x] #3 A test asserts the path appears in the rendered error chain for at least one of the three edges (e.g. atomic_write into a read-only directory)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Added io_at(op, path, err) in ingestor.rs: rewraps std::io::Error preserving ErrorKind and prefixing "<operation> <path>". Applied to all three edges (creating metadata ingest directory / running `cargo metadata` in working directory / writing staged cargo metadata JSON), so the three render distinguishably. New test metadata_collect_io_error_names_the_offending_path drives the create_dir_all edge (data dir under a regular file) and asserts both the path and the operation appear in the rendered chain. Variant stays DbError::Io so the TASK-1546 External-vs-Io test contract is unchanged.
<!-- SECTION:NOTES:END -->
