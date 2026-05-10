---
id: TASK-0907
title: 'TRAIT-9: ProgressDisplay !Send marker is brittle, undiscoverable invariant'
status: Done
assignee: []
created_date: '2026-05-02 10:10'
updated_date: '2026-05-02 11:21'
labels:
  - code-review-rust
  - structure
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/display.rs:95`

**What**: A PhantomData pointer marker is used to make ProgressDisplay !Send so handle_event cannot be polled on a tokio task. The marker is structural but undiscoverable: a future refactor swapping the synchronous tap writer for a Send async writer must remember to also remove this marker, otherwise the type stays !Send and nothing flags the over-restriction.

**Why it matters**: The invariant is encoded subtly; removing the dependency it guards leaves a phantom-only constraint that costs flexibility for no documented benefit.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Document the exact field that is non-Send today, and tie the marker to that field rather than a free-floating phantom
- [ ] #2 Compile-fail test asserts ProgressDisplay: !Send, with a comment naming the contributing field
<!-- AC:END -->
