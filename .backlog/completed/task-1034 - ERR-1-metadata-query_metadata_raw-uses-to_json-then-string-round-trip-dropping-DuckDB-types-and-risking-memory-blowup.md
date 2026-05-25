---
id: TASK-1034
title: >-
  ERR-1: metadata query_metadata_raw uses to_json then string round-trip,
  dropping DuckDB types and risking memory blowup
status: Done
assignee: []
created_date: '2026-05-07 20:24'
updated_date: '2026-05-07 23:30'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/lib.rs:186-196`

**What**: `query_metadata_raw` reads the singleton `metadata_raw` row by selecting `to_json(m)::VARCHAR` into a Rust `String`, then `serde_json::from_str` parses it back into a `serde_json::Value`. For a workspace whose `cargo metadata` output is multi-megabyte (this repo's own metadata.json is already ~1MB; large workspaces hit 10+MB), this allocates the full JSON twice (once as VARCHAR row, once as parsed Value) plus the DuckDB native columnar buffers — three live copies during the round-trip.

The cargo-metadata bytes are already on disk in `data_dir/metadata.json`. The provider could short-circuit by reading the file directly when the on-disk staging is fresher than the DB ingest, or DuckDB could `COPY` the row via `SELECT ... TO 'metadata.json'` to avoid the in-memory `String` materialisation. As written, very large workspaces can OOM the `ops about` process at the `query_row<String>` step.

**Why it matters**: TASK-0926 already added a SEC-33 byte cap on Cargo.toml reads to prevent OOM via oversized manifests; this is the symmetric exposure on the cargo-metadata path. It's not adversarial — large legitimate workspaces hit this naturally — so it's a correctness issue rather than security.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 query_metadata_raw either streams the JSON without materialising a single String or imposes a documented size cap with a clear error
- [ ] #2 Memory profile for a synthetic 50MB metadata fixture stays bounded (decision documented in the task)
<!-- AC:END -->
