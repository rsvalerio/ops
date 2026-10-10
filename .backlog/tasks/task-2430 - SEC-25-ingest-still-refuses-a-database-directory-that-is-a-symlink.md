---
id: TASK-2430
title: 'SEC-25: ingest still refuses a database directory that is a symlink'
status: Done
assignee: []
created_date: '2026-10-04 15:28'
updated_date: '2026-10-10 14:46'
labels:
  - code-review-rust
  - SEC
dependencies: []
parent_task_id: 'TASK-2440'
modified_files:
  - extensions/sqlite/src/sql/ingest/dir.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/sqlite/src/sql/ingest/dir.rs` (`create_ingest_dir`, the `reject_untrusted_ingest_dir(parent)` call)

**What**: TASK-2412 stopped ingest from chmodding or refusing the database's parent directory on its mode, but kept the pre-existing rule that the *immediate* parent must not be a symlink. A user whose `target/ops` (or configured `data.path` directory) is a symlink, e.g. to another disk, gets `ingest dir ... is a symlink; refusing to stage data through it`, although `Sqlite::open` follows the same symlink for the database file itself.

**Why it matters**: Same class as TASK-2412: a directory the tool does not own conceptually makes ingestion fail. The check is also an lstat followed by a path-based mkdir, so its security value is limited now that `IngestDir` verifies owner, mode and identity of the leaf. Needs a decision: drop the parent check, or keep it and document it.

**Origin**: discovered during TASK-2416 while fixing TASK-2412.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Ingest with the database in a symlinked directory either works, or the refusal is a documented, tested decision

<!-- AC:END -->
