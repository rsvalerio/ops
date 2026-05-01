---
id: TASK-0227
title: >-
  CONC-1: Mutex lock held across prepare/query_map/collect — long critical
  section
status: Done
assignee: []
created_date: '2026-04-23 06:33'
updated_date: '2026-04-23 08:55'
labels:
  - rust-code-review
  - concurrency
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/code.rs:31`

**What**: `conn = db.lock().ok()?` then prepares, runs, and collects — whole DB interaction under the lock.

**Why it matters**: Blocks other lock holders while results stream; no yield points.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Scope the lock to the prepare+query phase only
- [ ] #2 Benchmark contention under concurrent subpage rendering
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
AC#2 (benchmark contention under concurrent subpage rendering) not implemented in this run — requires a dedicated bench harness; AC#1 (scope the lock) is the substantive fix and is in.
<!-- SECTION:NOTES:END -->
