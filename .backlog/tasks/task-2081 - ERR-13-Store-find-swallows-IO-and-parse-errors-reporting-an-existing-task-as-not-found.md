---
id: TASK-2081
title: 'ERR-13: Store::find swallows IO and parse errors, reporting an existing task as not found'
status: Done
assignee: []
created_date: '2026-09-07 22:58'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-rust
  - error-handling
dependencies: []
parent_task_id: 'TASK-2243'
modified_files:
  - crates/backlog/src/store.rs
priority: medium
ordinal: 9000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/backlog/src/store.rs:197`

**What**: `Store::find` skips every failure silently: `let Ok(read) = std::fs::read_dir(..) else { continue }`, `let Ok(src) = std::fs::read_to_string(..) else { continue }`, `let Ok(doc) = TaskDoc::parse(..) else { continue }`. Any error other than a genuinely absent task yields `None`, which `run_view`/`run_edit`/`run_wave_members` render as `task TASK-0059 not found`.

**Why it matters**: A permissions error or unreadable directory on the exact tree holding the task is reported as "not found" — a wrong diagnosis the operator cannot act on, and exactly the failure shape `scan_all_tasks` was deliberately hardened against (its non-NotFound `read_dir` errors propagate naming the directory, pinned by the tests `scan_all_read_failure_names_the_directory` and `scan_all_parse_failure_names_the_file`). `find` is the lone scan path that still degrades silently, and nothing in its doc comment says so. A parse-skipping reading is defensible for tolerance, but a `read_dir` failure other than `NotFound` should surface like its sibling scans do.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A read_dir failure other than NotFound propagates from Store::find (or a find_with seam) naming the directory, mirroring scan_all_tasks
- [ ] #2 The doc comment on Store::find states the tolerance rule for files that fail to read or parse
- [ ] #3 A test pins that an unreadable directory surfaces an error naming it rather than None
<!-- AC:END -->
