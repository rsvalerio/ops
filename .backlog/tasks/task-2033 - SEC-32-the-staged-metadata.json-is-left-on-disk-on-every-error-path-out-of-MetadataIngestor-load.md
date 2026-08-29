---
id: TASK-2033
title: >-
  SEC-32: the staged metadata.json is left on disk on every error path out of
  MetadataIngestor::load
status: To Do
assignee:
  - TASK-2042
created_date: '2026-08-28 21:59'
updated_date: '2026-08-29 11:35'
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
- [ ] #1 The staged metadata.json is removed on every exit from MetadataIngestor::load, success or failure, via a single Drop-based guard rather than repeated call sites
- [ ] #2 The reject_non_singleton path in particular leaves no metadata.json behind
- [ ] #3 A test drives load() to a failure (e.g. the two-row fixture) and asserts the staged file no longer exists
<!-- AC:END -->
