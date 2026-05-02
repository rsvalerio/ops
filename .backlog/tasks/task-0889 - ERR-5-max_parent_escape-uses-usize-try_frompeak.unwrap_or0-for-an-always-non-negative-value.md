---
id: TASK-0889
title: >-
  ERR-5: max_parent_escape uses usize::try_from(peak).unwrap_or(0) for an
  always-non-negative value
status: Triage
assignee: []
created_date: '2026-05-02 09:38'
labels:
  - code-review-rust
  - ERR
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/hook-common/src/git.rs:135`

**What**: `max_parent_escape` computes `peak: i64` as the maximum positive value of `-depth` and returns `usize::try_from(peak).unwrap_or(0)`. The construction guarantees `peak >= 0` (every assignment is `if -depth > peak { peak = -depth }`, and `peak` initialises to 0), so the `unwrap_or(0)` branch is unreachable on every input — but the fallback silently masks any future refactor that breaks the invariant by reporting "no escape" for what would actually be an arithmetic surprise.

**Why it matters**: ERR-5 / unhelpful fallback. Because the function is the gate the SEC-14 traversal cap depends on, "silently report 0" on an unexpected input is the worst possible failure mode — it claims a pointer is safe when invariants have been violated. Either (a) prove the invariant in the type system by tracking peak as `usize` directly (clamp at the assignment site), or (b) `.expect("invariant: peak is non-negative")` so an invariant breach is loud, not silent.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 fallback either removed by tracking peak as usize, or replaced with an .expect that documents the invariant
- [ ] #2 behavior on existing inputs (cancellation patterns, deep traversal) preserved
<!-- AC:END -->
