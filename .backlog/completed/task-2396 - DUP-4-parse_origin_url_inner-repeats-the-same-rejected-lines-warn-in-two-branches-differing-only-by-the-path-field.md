---
id: TASK-2396
title: 'DUP-4: parse_origin_url_inner repeats the same rejected-lines warn in two branches differing only by the path field'
status: Done
assignee: []
created_date: '2026-10-04 14:15'
updated_date: '2026-10-04 15:30'
labels:
  - code-review-rust
  - DUP
dependencies: []
parent_task_id: 'TASK-2421'
modified_files:
  - extensions/git/src/config.rs
priority: low
ordinal: 1000
dedup_key: 'DUP-4:extensions/git/src/config.rs:parse_origin_url_inner'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/git/src/config.rs:parse_origin_url_inner` (the `if let Some(p) = path { warn!(path = ?p, ...) } else { warn!(...) }` block)

**What**: Two near-identical `tracing::warn!` calls with the same message; one adds `path`.

**Why it matters**: Message text must be kept in sync in two places. Log once with `path = ?path` (tracing records `Option` via Debug) or build the event once.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The rejected-url warning is emitted from a single call site, including the path when known

<!-- AC:END -->
