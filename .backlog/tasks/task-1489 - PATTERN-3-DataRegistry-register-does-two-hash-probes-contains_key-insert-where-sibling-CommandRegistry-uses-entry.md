---
id: TASK-1489
title: >-
  PATTERN-3: DataRegistry::register does two hash probes (contains_key + insert)
  where sibling CommandRegistry uses entry()
status: To Do
assignee:
  - TASK-1579
created_date: '2026-05-18 16:17'
updated_date: '2026-05-19 16:46'
labels:
  - code-review-rust
  - pattern
  - perf
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/extension/src/data.rs:224-244`

**What**: `DataRegistry::register` does `self.providers.contains_key(&name)` then `self.providers.insert(name, provider)` — two hash probes per registration on the happy path. The sibling `CommandRegistry::insert` (extension.rs:125-143) was migrated to `IndexMap::entry()` under PATTERN-3 / TASK-0753 so the hot path consults the map exactly once, with the audit-trail clone reusing the already-stored key. `DataRegistry::register` has the same shape (first-write-wins audit-trailed insert) but did not get the same treatment, leaving an inconsistency between the two registries.

**Why it matters**: Minor extra work per provider registration, and — more importantly — the asymmetry contradicts the cross-reference in the existing comments that pitch the two registries as mirror images of each other (CL-5 / TASK-0661, CL-5 / TASK-0756). A future reader comparing them will assume the optimization was intentional or, worse, copy the older pattern back into `CommandRegistry`. Aligning the implementations restores parity.

<!-- scan confidence: candidates to inspect -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 DataRegistry::register routes through IndexMap::entry()
- [ ] #2 happy path consults the inner map exactly once
- [ ] #3 duplicate-insert audit trail still records the rejected name (first-write-wins semantics preserved)
- [ ] #4 existing tests in crates/extension/src/tests.rs continue to pass
<!-- AC:END -->
