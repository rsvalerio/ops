---
id: TASK-1649
title: >-
  READ-4: Misplaced doc comment claims pure compute_channel_capacity is a
  memoised function
status: Done
assignee: []
created_date: '2026-05-29 19:05'
updated_date: '2026-06-07 11:32'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/parallel.rs:83-97`

**What**: The doc comment block above `compute_channel_capacity` opens with two lines that belong to a different function:

```
/// PERF-3 / TASK-1171: memoised sibling of [`resolve_max_parallel`]. Same
/// caching contract.
/// PATTERN-1 (TASK-1236): mpsc channel capacity for a parallel plan.
...
pub(crate) fn compute_channel_capacity(...) -> usize {
    let live_producers = steps_len.min(max_parallel).max(1);
    live_producers.saturating_mul(event_budget)
}
```

`compute_channel_capacity` is a **pure, non-memoised** function — it touches no `OnceLock` and has no caching contract. The "memoised sibling of `resolve_max_parallel`. Same caching contract." sentence describes `resolve_event_budget` (defined just below at parallel.rs:99), which is the actual `OnceLock`-memoised sibling and currently carries **no** doc comment at all. The two lines were evidently meant to sit above `resolve_event_budget` and landed on the wrong function.

**Why it matters**: A reader scanning the public-ish helper surface is told a pure arithmetic helper is memoised with a caching contract, which is false and could lead a future caller to assume calls are deduped/cached (and to skip adding caching where it is actually needed). Meanwhile the genuinely memoised `resolve_event_budget` is undocumented, so its first-call-parse / cached-thereafter contract — which matters because env mutations after the first call are ignored — is invisible at its definition. READ-4: documentation must match the code it annotates.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The 'memoised sibling of resolve_max_parallel. Same caching contract.' doc lines no longer sit above compute_channel_capacity
- [x] #2 compute_channel_capacity's doc describes only its pure min(steps_len, max_parallel)*event_budget behaviour with no caching claim
- [x] #3 resolve_event_budget carries a doc comment describing its OnceLock memoisation / first-call-parse contract
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Moved the misplaced memoised-sibling doc lines off compute_channel_capacity (now documented as pure arithmetic, no caching claim) and gave resolve_event_budget a doc comment describing its OnceLock first-call-parse / cached-thereafter contract.
<!-- SECTION:NOTES:END -->
