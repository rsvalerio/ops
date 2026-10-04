---
id: TASK-2412
title: 'SEC-25: create_ingest_dir changes permissions on the database''s parent directory, which can be the workspace root'
status: To Do
assignee: []
created_date: '2026-10-04 14:18'
updated_date: '2026-10-04 14:52'
labels:
  - code-review-rust
  - SEC
dependencies: []
parent_task_id: 'TASK-2416'
modified_files:
  - extensions/sqlite/src/sql/ingest/dir.rs
  - extensions/sqlite/src/connection.rs
priority: medium
ordinal: 1000
dedup_key: 'SEC-25:extensions/sqlite/src/sql/ingest/dir.rs:harden_ingest_parent'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/sqlite/src/sql/ingest/dir.rs:96-120` (`create_ingest_dir`) and `harden_ingest_parent` (~L170-250), `extensions/sqlite/src/connection.rs:130` (`resolve_path`)

**What**: The ingest dir is `<db_path>.ingest`, so its parent is the database's directory. `harden_ingest_parent` clears group/other write bits (0o775 to 0o755) on that directory, or refuses to stage if it is not owned by us and not sticky. With the default `target/ops/data.db` this is harmless. With `data.path = "data.db"` (resolved relative to the workspace root) or any shared directory, ingest chmods the user's project directory or fails with `PermissionDenied`.

**Why it matters**: A cache write silently changes permissions on a directory the tool does not own conceptually, or fails ingestion for group-writable project directories.

**Fix direction**: confine hardening to a dedicated ops-owned subdirectory, or skip parent hardening when the parent is not one ops created, relying on the anchored `IngestDir` for the swap defense.

<!-- scan confidence: candidates to inspect -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Ingest with a database in a user-owned, group-writable directory does not modify that directory's mode and does not fail
- [ ] #2 A test covers a db path whose parent is group-writable
<!-- AC:END -->
