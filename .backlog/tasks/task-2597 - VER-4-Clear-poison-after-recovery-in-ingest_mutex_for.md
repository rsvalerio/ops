---
id: TASK-2597
title: 'VER-4: Clear poison after recovery in ingest_mutex_for'
status: Done
assignee: []
created_date: '2026-10-10 15:47'
updated_date: '2026-10-10 21:58'
labels:
  - code-review-rust
  - ver
dependencies: []
parent_task_id: 'TASK-2625'
modified_files:
  - extensions/sqlite/src/connection.rs
priority: low
ordinal: 1000
dedup_key: 'VER-4:extensions/sqlite/src/connection.rs:ingest_mutex_for'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/sqlite/src/connection.rs:208-218`

**What**: `ingest_mutex_for` recovers a poisoned registry lock via `unwrap_or_else(|poisoned| { tracing::warn!(...); poisoned.into_inner() })` (line 209-212), which is the correct recovery — but it never calls `Mutex::clear_poison`. The same is true of the `ingest_locks` map mutex this guard belongs to: once poisoned, every subsequent `lock()` on that mutex returns `Err(PoisonError)` even though recovery has already decided the invariant is re-establishable.

**Why it matters**: VER-4: `Mutex::clear_poison` has been stable since Rust 1.77 (workspace MSRV is 1.97), and the rule says to call it on the recovery path when the lock invariant is re-established by the recovery itself. Without it, the `tracing::warn!` breadcrumb fires on *every* subsequent acquire for the rest of the process lifetime — one panic turns into permanent log spam that devalues the warn as an audit signal. Fix: after taking `into_inner`, call `self.ingest_locks.clear_poison()` before proceeding (or use `poisoned.clear_poison()` style via the exposed API on the lock).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 After a poison recovery in ingest_mutex_for, the ingest_locks mutex is cleared via clear_poison so later acquires succeed without re-warning; a test pins that the recovery warn fires once, not per acquire

<!-- AC:END -->
