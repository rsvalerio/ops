---
id: TASK-2033
title: >-
  SEC-32: the staged metadata.json is left on disk on every error path out of
  MetadataIngestor::load
status: Done
assignee:
  - TASK-2042
created_date: '2026-08-28 21:59'
updated_date: '2026-08-29 12:42'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - extensions-rust/metadata/src/ingestor.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/ingestor.rs:50-89`

**What**: `cleanup_staged_file(&path)` is called at exactly one place — line 87,
immediately before `Ok(LoadResult::success(...))`. Every other exit from `load`
returns via `?` or an explicit `Err` and skips it, so the staged
`data_dir/metadata.json` survives:

- `init_schema` failure
- `build_views` failure (`metadata_raw create` / `crate_dependencies view`)
- `query_record_count` failure
- the new `reject_non_singleton` rejection added by ERR-1 / TASK-1891
- `extract_workspace_root` failure
- `checksum_file` / `upsert_data_source` failure

TASK-1891 made the non-singleton case a hard error precisely so the next run
re-ingests; that new error path is now the most likely way to leave the file
behind.

**Why it matters**: SEC-32 — the same class already filed against the terraform
pipeline as TASK-1927 ("plan artifacts are left on disk on every error path
after the plan runs"). The file is a full `cargo metadata` dump: it names every
workspace member, every dependency, and absolute local filesystem paths, and it
is written with `atomic_write`'s default permissions. Leaving it after a failed
ingest is both an unbounded on-disk residue and a wider disclosure window than
the successful path allows itself. The fix is structural, not a fifth call site:
a guard whose `Drop` removes the staged file unless the load is explicitly
marked successful, mirroring the terraform `cleanup_artifacts` shape.

**Origin**: discovered during TASK-1999 (code-review-plan-wave165) while fixing
TASK-1891, whose new hard-error path made the gap reachable.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The staged metadata.json is removed on every exit from MetadataIngestor::load, success or failure, via a single Drop-based guard rather than repeated call sites
- [x] #2 The reject_non_singleton path in particular leaves no metadata.json behind
- [x] #3 A test drives load() to a failure (e.g. the two-row fixture) and asserts the staged file no longer exists
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Fixed in code-review/TASK-2042 (wave TASK-2042).

AC#1: added a `StagedFile` guard in extensions-rust/metadata/src/ingestor.rs,
armed before `init_schema` (the first fallible step) and holding the staged path
for the whole of `load`. Its `Drop` calls `cleanup_staged_file`, so success, every
`?` and the explicit `reject_non_singleton` return all clean up through one code
path; the pre-`Ok` call site is gone. Shape mirrors the terraform pipeline's
SEC-32 / TASK-1927 cleanup, using `Drop` rather than a wrapper because `load`'s
early exits are `?` rather than one fallible expression.

`cleanup_staged_file` now swallows `NotFound` (SEC-25: no check-then-act probe)
because cleanup can legitimately run when `collect` never staged a file — a
warning there would be noise, not signal.

AC#2 + AC#3: `metadata_load_rejection_removes_the_staged_json` drives `load` to
the non-singleton rejection with the two-row fixture and asserts the file is
gone; `metadata_load_removes_the_staged_json_when_the_table_build_fails` covers a
failure *earlier* than the row count (unparseable staged JSON), which is what
pins the cleanup as a scope guard rather than one more call site.

`ops verify` 7/7 clean; `cargo nextest run --workspace --all-features` 2899
passed.
<!-- SECTION:NOTES:END -->
