---
id: TASK-2832
title: 'CONC-16: per-table ingest locks recover through poisoned mutexes via into_inner and clear_poison'
status: Done
assignee: []
created_date: '2026-11-18 18:50'
updated_date: '2026-11-18 21:54'
labels:
  - code-review-rust
  - CONC
dependencies: []
parent_task_id: 'TASK-2839'
modified_files:
  - extensions/sqlite/src/sql/ingest/orchestrator.rs
  - extensions/sqlite/src/connection.rs
priority: medium
ordinal: 1000
dedup_key: 'CONC-16:extensions/sqlite/src/sql/ingest/orchestrator.rs:provide_via_ingestor'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/sqlite/src/sql/ingest/orchestrator.rs:93-104` (`provide_via_ingestor`), `extensions/sqlite/src/connection.rs:198-213` (`Sqlite::ingest_mutex_for`)

**What**: Both locks on the ingest path bypass poisoning. `ingest_mutex_for` does `self.ingest_locks.lock().unwrap_or_else(|poisoned| { …; self.ingest_locks.clear_poison(); poisoned.into_inner() })` on the per-instance registry `Mutex<HashMap<&'static str, Arc<Mutex<()>>>>`. `provide_via_ingestor` does the same on the per-table `Mutex<()>`, then carries on into drop / collect / load. The comments justify this with "the mutex only guards a `()`" and cite VER-4, which is retired in favour of CONC-16.

The per-table mutex does not guard just a `()`: it serialises the drop → probe → collect → load → query critical section over the database and the ingest directory. A panic that poisons it happened mid-ingest, where the table may be dropped or half-loaded and staged files may be half-cleaned. The next caller clears the flag and proceeds as if nothing happened. `Sqlite::lock` already takes the opposite stance for the connection mutex (poison surfaces as `DbError::MutexPoisoned`), so the crate has two policies for one critical section.

**Why it matters**: CONC-16 forbids recovering through a poisoned lock. "Guards no multi-step invariant" is the claim the rule says nobody re-checks, and here it is already untrue of the section the lock protects.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Neither ingest_mutex_for nor provide_via_ingestor calls PoisonError::into_inner or clear_poison; poisoning propagates (DbError::MutexPoisoned / an error from provide_via_ingestor, or .expect with a message)
- [x] #2 The poison-recovery tests (e.g. ingest_mutex_for_clears_poison_so_recovery_warns_once) are replaced by tests asserting the poisoned lock surfaces as an error

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.30 (code-review-rust run, 2026-11-18).

ingest_mutex_for now returns DbResult and maps poison to DbError::MutexPoisoned. provide_via_ingestor maps the per-table lock poison to DbError::MutexPoisoned, with context naming the table. Replaced tests: ingest_mutex_for_surfaces_a_poisoned_registry_as_an_error, and panic_in_collect_surfaces_as_poisoned_ingest_lock_error, which replaces panic_in_collect_does_not_brick, poison_recovery_emits_warn_log and poison_recovery_warns_once. The rebase conflicted with TASK-2853's ops-sqlite rewrite; it was resolved keeping 2853's wording and changes.

<!-- SECTION:NOTES:END -->
