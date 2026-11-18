---
id: TASK-2835
title: 'READ-6: Sqlite::open honours SQLite URI filenames while open_readonly does not'
status: To Do
assignee: []
created_date: '2026-11-18 18:51'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - READ
dependencies: []
parent_task_id: 'TASK-2853'
modified_files:
  - extensions/sqlite/src/connection.rs
priority: low
ordinal: 1000
dedup_key: 'READ-6:extensions/sqlite/src/connection.rs:Sqlite::open'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/sqlite/src/connection.rs:85-101` (`Sqlite::open`) vs `:119-134` (`Sqlite::open_readonly`)

**What**: `open` passes `rusqlite::OpenFlags::default()`, which in rusqlite 0.41 is `READ_WRITE | CREATE | NO_MUTEX | URI`. `open_readonly` builds its flags explicitly without `SQLITE_OPEN_URI`, and its comment says "URI is off, so a literal path is taken as-is". So the same `&Path` names different things depending on the opener. A path starting with `file:` is parsed by `open` as a URI, where query parameters such as `mode=memory`, `vfs=…`, `immutable=1` or `nolock=1` change behaviour, and by `open_readonly` as a literal file. `open` is public and takes any `&Path`. Today `resolve_path` always produces an absolute or root-joined path, so a `file:` prefix is not reachable from config, but that is an unstated invariant of the caller.

**Why it matters**: The two openers interpret the same input inconsistently, and a stated security posture ("literal path taken as-is") holds for only one of them.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Sqlite::open builds explicit flags without SQLITE_OPEN_URI (READ_WRITE | CREATE | NO_MUTEX), matching open_readonly's literal-path semantics
- [ ] #2 A test opens a path beginning with 'file:' via Sqlite::open and asserts a literal file of that name is created
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.30 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
