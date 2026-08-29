---
id: TASK-2067
title: >-
  SEC-25: DuckDB reads the staged ingest JSON by path, the one edge the
  IngestDir anchor cannot cover
status: Triage
assignee: []
created_date: '2026-08-29 18:09'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - extensions/duckdb/src/sql/ingest/sql.rs
  - extensions/duckdb/src/sql/ingest/dir.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/ingest/sql.rs`, `extensions/duckdb/src/sql/ingest/dir.rs`

**What**: TASK-2054 anchored every staged *write*, rename, unlink and checksum on
a verified directory descriptor. One edge is deliberately left by-name:
`create_table_from_json_sql` interpolates the staged file's path into
`read_json_auto('<path>')`, and the embedded DuckDB engine takes a path string
with no descriptor-passing API, so the *read* of the staged JSON still resolves
the ingest directory by name. `IngestDir::path` / `entry_path` exist for exactly
this, and the type's docs say so.

**Why it matters**: the residual is narrower than the write side — an attacker
who swaps the directory name between the anchored write and the DuckDB read can
feed the engine JSON of their choosing, but cannot capture what ops staged.
Whether that is acceptable is a judgement the finding should record explicitly
rather than leave implicit in a doc comment. Options if it is not: verify the
file's `(dev, ino)` through the anchor immediately before handing DuckDB the
path (shrinks, does not close, the window), read the staged JSON through the
anchor and feed DuckDB an in-memory value or a `/proc/self/fd`-style path on
platforms that offer one, or accept and document it as the engine's boundary.

**Origin**: discovered during TASK-2063 while fixing TASK-2054.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The by-path read of the staged ingest JSON is either hardened (identity re-checked through the anchor, or the read routed through it) or explicitly accepted with the reasoning recorded next to create_table_from_json_sql
<!-- AC:END -->
