---
id: TASK-1867
title: >-
  READ-5: data_dir_for_db turns an in-memory DB into a relative
  ':memory:.ingest' dir in the process CWD — the debris is committed to the repo
status: Done
assignee:
  - TASK-2006
created_date: '2026-08-27 15:30'
updated_date: '2026-08-28 22:15'
labels:
  - code-review-rust
  - correctness
dependencies: []
modified_files:
  - extensions/duckdb/src/sql/ingest/dir.rs
  - extensions/duckdb/src/sql/ingest/orchestrator.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/ingest/dir.rs:8-12` (`data_dir_for_db`), consumed at `extensions/duckdb/src/sql/ingest/orchestrator.rs:122`; artifact at `extensions/duckdb/:memory:.ingest/counting.json`

**What**: `data_dir_for_db` blindly string-appends `.ingest` to whatever `db.path()` returns:

```rust
let mut path = db_path.as_os_str().to_os_string();
path.push(".ingest");
```

`DuckDb::open_in_memory` sets `db_path` to the sentinel `PathBuf::from(":memory:")` (`connection.rs:504`), so for an in-memory database this yields the **relative** path `:memory:.ingest`. `provide_via_ingestor` then calls `create_ingest_dir(&data_dir)` on it, which `create_dir_all`s and writes staged JSON into a `:memory:.ingest/` directory inside whatever the process's current working directory happens to be — the user's project root when `ops` runs, or the crate directory when tests run.

That is not hypothetical: `extensions/duckdb/:memory:.ingest/counting.json` (containing `[{"id": 1}]`, the payload written by the `CountingIngestor` test double in `orchestrator.rs`) is **tracked in git** — `git ls-files` lists it. A past test run against an in-memory DB littered the crate directory and the debris was committed.

`DuckDb::open_in_memory` is `pub`, `ctx.db` can carry one, and `provide_via_ingestor` never checks — so a caller that hands the ingest pipeline an in-memory handle silently gets filesystem side effects in `$PWD` instead of the isolated staging area the design assumes.

**Why it matters**: The sentinel `:memory:` is a DuckDB connection string, not a filesystem path; treating it as one produces a junk directory in the user's working tree, breaks the "staging lives under `target/ops/`" contract that the SEC-25 `0o700` hardening and the operator-audit note in `load_with_sidecar` both rely on, and has already polluted the repository.

**Related**: this is the same path-shape assumption that `default_db_path`/`resolve_path` guarantee for real databases; only the in-memory sentinel escapes it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 data_dir_for_db (or provide_via_ingestor) rejects or redirects the ':memory:' sentinel instead of deriving a relative CWD path from it — e.g. a typed error, or staging under a tempdir
- [x] #2 No code path can create a ':memory:.ingest' directory in the process working directory
- [x] #3 extensions/duckdb/:memory:.ingest/ is removed from git and a guard (gitignore entry or test) keeps it from returning
- [x] #4 A test drives provide_via_ingestor with an in-memory DuckDb and asserts no directory is created in the CWD
<!-- AC:END -->
