---
id: TASK-1411
title: >-
  PERF-3: Variables::expand_warn_seen HashSet grows unbounded on adversarial
  input
status: To Do
assignee:
  - TASK-1454
created_date: '2026-05-13 18:17'
updated_date: '2026-05-13 19:09'
labels:
  - code-review-rust
  - perf
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/expand.rs:135`

**What**: `expand_warn_seen` is a process-global `Mutex<HashSet<String>>` that inserts every distinct `var_name` whose expansion fails. An adversarial config or env that triggers expand failures across many distinct variable names (e.g. composite commands with `${VAR_001}`..`${VAR_NNNN}` references) will grow the set unbounded.

**Why it matters**: Tracking warned-once-per-name is correct, but the unbounded `String` key set is a slow memory leak for any long-lived embedder or test harness that reuses the process. Cap the set size or evict LRU.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 cap expand_warn_seen size (e.g. 256 distinct names) and document the cap in the dedup contract
- [ ] #2 evict oldest entries when at cap so new distinct vars still surface a single user-facing warn
- [ ] #3 regression test inserting >cap distinct names asserts memory stays bounded and warn-once-per-distinct holds within the cap window
<!-- AC:END -->
