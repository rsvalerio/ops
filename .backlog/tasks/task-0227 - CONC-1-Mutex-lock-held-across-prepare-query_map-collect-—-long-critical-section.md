---
id: TASK-0227
title: >-
  CONC-1: Mutex lock held across prepare/query_map/collect — long critical
  section
status: To Do
assignee: []
created_date: '2026-04-23 06:33'
updated_date: '2026-04-23 06:45'
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
- [ ] #1 Scope the lock to the prepare+query phase only
- [ ] #2 Benchmark contention under concurrent subpage rendering
<!-- AC:END -->
