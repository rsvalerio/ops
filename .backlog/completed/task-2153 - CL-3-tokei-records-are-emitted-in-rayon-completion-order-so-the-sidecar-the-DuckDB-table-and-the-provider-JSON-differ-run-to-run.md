---
id: TASK-2153
title: 'CL-3: tokei records are emitted in rayon completion order, so the sidecar, the DuckDB table and the provider JSON differ run-to-run'
status: Done
assignee: []
created_date: '2026-09-08 07:03'
updated_date: '2026-09-09 18:36'
labels:
  - code-review-rust
  - pattern
dependencies: []
parent_task_id: 'TASK-2243'
modified_files:
  - extensions/tokei/src/lib.rs
  - extensions/tokei/src/tests.rs
priority: medium
ordinal: 66000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/tokei/src/lib.rs:379` (`flatten_tokei_records`), `extensions/tokei/src/lib.rs:91` (`query_tokei_files`)

**What**: `flatten_tokei_records` iterates `Languages` (a `BTreeMap`, ordered) but emits `language.reports` in stored order. Tokei fills that `Vec` from `utils::fs::get_all_files`, which drives a `crossbeam` channel through `par_bridge()` and calls `entry.add_report(stats)` from arbitrary rayon workers (tokei 14.0.0, `src/utils/fs.rs:88-108`). The push order is therefore worker-scheduling dependent, and nothing downstream re-imposes an order: `scan_tokei` returns the `Vec` as-is, `collect_sidecar` serialises it straight into `tokei_files.json`, `read_json_auto` loads it into `tokei_files` in file order, and `query_tokei_files` does `SELECT ... FROM tokei_files` with no `ORDER BY`.

The sibling extension already treats this as a defect and fixes it: `extensions-rust/loc/src/lib.rs` sorts by `row_key` before returning, with the rationale "Workers finish in arbitrary order. Sorting keeps the JSON sidecar and the DuckDB ingest byte-stable across runs, so a diff of two collections shows real changes only." tokei has the same parallel fill and no such sort.

**Why it matters**: three concrete consequences on an unchanged tree —
1. The provider's JSON array (what a consumer or a prompt sees) lists the same files in a different order on every collection, so no two runs are comparable and no output can be snapshot-tested.
2. `SidecarIngestorConfig::load_with_sidecar` SHA-256s `tokei_files.json` and writes it to `data_sources.checksum`; a churning checksum makes that column useless as a change signal (`get_source_checksum` cannot distinguish "the repo changed" from "the workers finished in a different order").
3. Row order in `tokei_files` — and therefore any unordered `SELECT` against it — is unstable, which is precisely the non-determinism `extensions-rust/loc` documented and removed.

<!-- scan confidence: verified against tokei 14.0.0 source, not a grep heuristic -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 scan_tokei (or flatten_tokei_records) sorts records by a total key — file path, with language as a tiebreak — before they are returned, matching the row_key policy in extensions-rust/loc/src/lib.rs
- [x] #2 query_tokei_files carries an explicit ORDER BY so the queried path is ordered too, not only the ingested one
- [x] #3 A test builds a multi-file, multi-language fixture and asserts the exact record sequence (not just the count), so a regression to worker order fails the suite
- [x] #4 The chosen ordering is documented next to the sort, referencing the byte-stable-sidecar rationale

<!-- AC:END -->
