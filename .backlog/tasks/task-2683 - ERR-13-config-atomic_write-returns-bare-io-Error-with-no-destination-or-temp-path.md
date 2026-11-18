---
id: TASK-2683
title: 'ERR-13: config::atomic_write returns bare io::Error with no destination or temp path'
status: Done
assignee: []
created_date: '2026-11-18 18:27'
updated_date: '2026-11-18 22:14'
labels:
  - code-review-rust
  - ERR
dependencies: []
parent_task_id: 'TASK-2841'
modified_files:
  - crates/core/src/config/edit.rs
priority: low
ordinal: 1000
dedup_key: 'ERR-13:crates/core/src/config/edit.rs:atomic_write'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/edit.rs:193-207` (`atomic_write`), with `write_tmp_and_sync` (`:312-347`) and `resolve_parent_and_filename` (`:212-221`)

**What**: `atomic_write` is `pub` and returns `std::io::Result<()>`; every failure (temp `open` with `create_new`, `set_permissions`, `write_all`, `sync_all`, `rename`, and the "path has no file name" case) propagates the raw `io::Error`, which names neither `path` nor the temp file. Only `write_ops_toml` wraps it with context; other callers propagate it bare — e.g. `crates/backlog/src/cmd/edit.rs:67` (`atomic_write(&entry.path, &rendered)?`), `crates/cli/src/init_cmd.rs:169`, `extensions/sqlite/src/sql/ingest/dir.rs:412` — so a failure surfaces as `Permission denied (os error 13)` with no file named.

**Why it matters**: A failed atomic rewrite in a CI log or bug report cannot be traced to the file (or to the sibling temp file in a read-only directory, the usual cause) without re-running under strace.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Every error atomic_write returns names the destination path, and the temp path when the failing step touched it, while preserving the original io::ErrorKind
- [x] #2 A test asserts the error message of a failing atomic_write (e.g. unwritable parent directory) contains the destination path

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.03 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
