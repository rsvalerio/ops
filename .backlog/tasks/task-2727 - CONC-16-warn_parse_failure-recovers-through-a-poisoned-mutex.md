---
id: TASK-2727
title: 'CONC-16: warn_parse_failure recovers through a poisoned mutex'
status: To Do
assignee: []
created_date: '2026-11-18 18:35'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - concurrency
dependencies: []
parent_task_id: 'TASK-2839'
modified_files:
  - extensions-node/about/src/package_json.rs
priority: low
ordinal: 1000
dedup_key: 'CONC-16:extensions-node/about/src/package_json.rs:warn_parse_failure'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-node/about/src/package_json.rs:193` (`warn_parse_failure`)

**What**: The process-wide warn-once set `WARNED: OnceLock<Mutex<HashSet<PathBuf>>>` is locked with `.lock().unwrap_or_else(PoisonError::into_inner)`, so the function keeps using the guarded set after another thread panicked while holding the lock.

**Why it matters**: CONC-16 forbids recovering through a poisoned lock. The argument that this lock guards no multi-step invariant is not an exemption, because the critical section can grow later without anyone rechecking it. The workspace denies `clippy::expect_used`, so the fix needs a scoped `#[expect(clippy::expect_used, reason = "...")]`. Another option is a design that needs no lock, such as deduplicating at the `manifest_cache` layer.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 warn_parse_failure no longer calls PoisonError::into_inner; poisoning is propagated (expect with a message under a reasoned #[expect]) or the lock is removed
- [ ] #2 Existing warn-once test (malformed_package_json_warns_once_across_both_providers) still passes
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.09 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
