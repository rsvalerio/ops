---
id: TASK-2407
title: 'ERR-13: Filesystem errors in ops-sqlite reach callers without the path'
status: To Do
assignee: []
created_date: '2026-10-04 14:18'
updated_date: '2026-10-04 14:52'
labels:
  - code-review-rust
  - ERR
dependencies: []
parent_task_id: 'TASK-2416'
modified_files:
  - extensions/sqlite/src/connection.rs
  - extensions/sqlite/src/sql/ingest/dir.rs
  - extensions/sqlite/src/sql/ingest/sidecar.rs
priority: low
ordinal: 1000
dedup_key: 'ERR-13:extensions/sqlite/src/sql/ingest/dir.rs:IngestDir'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/sqlite/src/connection.rs:92` (`Sqlite::open`), `extensions/sqlite/src/sql/ingest/dir.rs` (`create_ingest_dir` ~L96-L120, `IngestDir::open`, `write_atomic`, `open_read`, `rename`, `remove_file`, `checksum`), `extensions/sqlite/src/sql/ingest/sidecar.rs` (`read_workspace_sidecar`)

**What**: `std::fs::create_dir_all(parent).map_err(DbError::Io)`, `DirBuilder::create`, `File::open`, `openat`/`renameat`/`unlinkat` failures and `read_to_end` errors are wrapped as bare `DbError::Io(io::Error)` with no path. `DbError::Io` renders as `IO error: Permission denied (os error 13)`. The orchestrator's context names the table and phase ("create ingest dir") but not the directory. Only `execute_json_load` adds the file name, and it does so by rebuilding the `io::Error` with `format!("...: {io}")`, which drops the original source.

**Why it matters**: In CI logs or user bug reports the failing file or directory cannot be identified.

<!-- scan confidence: candidates to inspect -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every DbError::Io produced from a filesystem call names the path or staged entry
- [ ] #2 Path context is attached without discarding the original io::Error as source
<!-- AC:END -->
