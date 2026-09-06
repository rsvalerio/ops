---
id: TASK-1864
title: >-
  SEC-12: load_with_sidecar takes create_sql/view_sql as bare &str, the one
  un-gated SQL entry point left in the ingest pipeline
status: Done
assignee:
  - TASK-2006
created_date: '2026-08-27 15:29'
updated_date: '2026-08-28 22:13'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - extensions/duckdb/src/ingestor.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/ingestor.rs:181-234` (`SidecarIngestorConfig::load_with_sidecar`, `create_tables_with`)

**What**: Every other identifier or fragment that reaches a formatted SQL string in this crate is gated by a construction-time newtype — `TableName::from_static` (const-validated), `ExtraOpts::new`, `QueryTableName` / `ColumnAlias` / `ColumnName`, `quoted_ident`. `load_with_sidecar` is the exception:

```rust
pub fn load_with_sidecar(&self, db: &DuckDb, data_dir: &Path, create_sql: &str, view_sql: &str)
...
conn.execute(create_sql, [])?;
conn.execute(view_sql, [])?;
```

Both arguments are arbitrary `&str` executed verbatim, and the three downstream call sites (`extensions/tokei/src/ingestor.rs:28`, `extensions-rust/loc/src/ingestor.rs:28`, `extensions-rust/test-coverage/src/ingestor.rs:29`) all pass a `String` built by `format!` in their own crate — i.e. the validated-builder discipline is enforced only by convention at sites this crate cannot see. `PerCrateI64Query::select_expr` solved exactly this problem by constraining the field to `&'static str` so "static-vetted SQL fragment" became a build-time property; nothing equivalent guards these two.

Note the two positional `&str` are also swappable (API-2): passing them in the wrong order creates the view first and produces a confusing `{name} create` error label.

**Why it matters**: This is the widest ungated path into `conn.execute` in the crate, and it sits on the ingest path that runs `CREATE OR REPLACE TABLE` against the project database. A future ingestor that interpolates a config-derived or metadata-derived value into its `create_sql` inherits zero of the crate's SEC-12 protections and nothing in this crate fails the build. Cross-crate note: the three real callers live in `extensions/tokei` and `extensions-rust/*`, but the missing gate is this crate's API contract.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 create_sql/view_sql are accepted through a validated newtype (e.g. a PreparedSql/IngestSql wrapper produced by create_table_from_json_sql and a view builder) rather than bare &str
- [x] #2 The two arguments are no longer interchangeable at the call site — swapping them is a type error
- [x] #3 The three downstream ingestors (tokei, rust loc, test-coverage) are migrated to the gated constructor
- [x] #4 A test pins that an unvalidated string cannot reach conn.execute through the public API
<!-- AC:END -->
