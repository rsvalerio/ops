---
id: TASK-0962
title: >-
  ARCH-2: typed_manifest_cache one-shot poison-warn loses signal on subsequent
  re-poisons
status: Done
assignee: []
created_date: '2026-05-04 21:47'
updated_date: '2026-05-04 22:51'
labels:
  - code-review-rust
  - observability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/query.rs:114-136` (lock_typed_manifest_cache)

**What**: On `PoisonError`, `lock_typed_manifest_cache` returns `into_inner()` and calls `cache.clear_poison()`, then logs via a `OnceLock`-gated warn that fires only once per process. Subsequent panics in another provider re-poison the lock; callers each pay the silent recovery branch because POISON_LOGGED already fired.

**Why it matters**: Repeat poisonings after the first are invisible. Operator sees one warn ever, then loses the signal for every subsequent panic in a different provider — defeats the "schema drift surfaces" intent.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either log every poison recovery (debug-level if warn-spam is the concern), or include a counter so observability isn't a one-shot
- [ ] #2 Test verifies a second-poison cycle produces an observable signal
<!-- AC:END -->
