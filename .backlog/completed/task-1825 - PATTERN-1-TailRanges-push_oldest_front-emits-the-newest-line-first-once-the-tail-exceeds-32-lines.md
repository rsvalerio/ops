---
id: TASK-1825
title: >-
  PATTERN-1: TailRanges::push_oldest_front emits the newest line first once the
  tail exceeds 32 lines
status: Done
assignee:
  - TASK-1984
created_date: '2026-08-27 11:33'
updated_date: '2026-08-29 00:36'
labels:
  - code-review-rust
  - correctness
dependencies: []
modified_files:
  - crates/core/src/output.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/output.rs:243-256`

**What**: `TailRanges` collects line ranges while walking an error buffer **backwards**, so every successive push is an *older* line than the last. `push_oldest_front` maintains that by shifting the inline array right and writing the new range at index 0, which makes `stack[0]` the oldest collected line and `stack[stack_len - 1]` the **newest**. `iter()` then emits `spill` followed by `stack[0..stack_len]`, i.e. oldest → newest. Correct while the array has room.

The overflow branch is not:

```rust
let overflow = self.stack[TAIL_STACK_CAP - 1];   // index 31 == the NEWEST entry
self.stack.copy_within(0..TAIL_STACK_CAP - 1, 1);
self.stack[0] = range;
self.spill.insert(0, overflow);                  // front of spill == absolute OLDEST
```

It evicts index `TAIL_STACK_CAP - 1` — the newest line — and inserts it at the front of `spill`, the slot reserved for the absolute oldest. The comment on the branch asserts the opposite ("the existing oldest entry rolls over into the spill").

Traced with `CAP = 3` and the backwards walk pushing `L5, L4, L3, L2, L1`:

| step | stack | spill |
|---|---|---|
| push L5 | `[L5]` | `[]` |
| push L4 | `[L4,L5]` | `[]` |
| push L3 | `[L3,L4,L5]` | `[]` |
| push L2 | `[L2,L3,L4]` | `[L5]` |
| push L1 | `[L1,L2,L3]` | `[L4,L5]` |

`iter()` yields **`L4, L5, L1, L2, L3`** instead of `L1..L5`. Not an off-by-one — the two newest lines are hoisted to the top of the rendered tail, and one more line migrates per push beyond the cap.

Secondary defect in the same branch: `spill.insert(0, …)` is O(len) per push, so the oversized path is O(n²) — the exact opposite of the allocation-avoidance goal stated at output.rs:200-205. Pushing to the back and reversing once (or `VecDeque::push_front`) fixes both defects together.

**Why it matters**: `format_error_tail` is `pub` in `ops_core::output` and is called from four out-of-crate extensions. Every in-repo caller passes `n = 5` or `n = 10`, so the bug is latent today — but the value is config-driven, and any caller that raises the tail past 32 silently gets a reordered error excerpt. Misordered failure output is worse than truncated output: the reader has no way to tell it is wrong.

<!-- scan confidence: verified by hand-tracing the algorithm at output.rs:243-256; no test in the file pushes past TAIL_STACK_CAP -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Once the inline array is full, a further (older) range goes to the front of the spill and the array is left untouched, so iter() yields buffer order for every n
- [x] #2 A test with n = 33 (the first overflow) and one with n = 40 assert format_error_tail returns the lines in exact buffer order
- [x] #3 The oversized path no longer front-inserts into a Vec per push, so collecting n ranges is O(n) rather than O(n squared)
- [x] #4 The comment on the overflow branch states the ordering invariant iter() depends on, and matches what the code does
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Fixed in wave TASK-1984. `TailRanges::push_oldest_front` now leaves the full inline array untouched and appends the older range to the spill (O(1) per push, no front-insert); `iter()` yields `spill.iter().rev()` then the inline entries, restoring buffer order for every n. The overflow-branch comment now states the ordering invariant iter() depends on. Tests: format_error_tail_preserves_buffer_order_at_first_overflow (n=33), format_error_tail_preserves_buffer_order_well_past_cap (n=40), and tail_ranges_overflow_appends_to_spill_and_leaves_stack_intact, which pins the O(n) shape structurally (stack_len == TAIL_STACK_CAP, spill holds the remainder) rather than by timing.
<!-- SECTION:NOTES:END -->
