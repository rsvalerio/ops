---
id: TASK-2820
title: 'READ-13: ops-tokei crate docs carry a migration guard for the removed load_tokei'
status: Done
assignee: []
created_date: '2026-11-18 18:48'
updated_date: '2026-11-18 22:52'
labels:
  - code-review-rust
  - readability
dependencies: []
parent_task_id: 'TASK-2855'
modified_files:
  - extensions/tokei/src/lib.rs
  - extensions/tokei/src/tests.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions/tokei/src/lib.rs:crate-root-docs'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/tokei/src/lib.rs:1-10` (crate-root `//!` docs and `compile_fail` doctest); related comment at `extensions/tokei/src/tests.rs:615-617`

**What**: The crate-level docs explain that the crate "exports no free `load_tokei` function" and include a `compile_fail` doctest (`let _ = ops_tokei::load_tokei;`) whose only job is to fail if that symbol comes back. This records the journey: a function was once removed in favour of `TokeiIngestor`. It does not describe the end state. A reader who joined after the removal gets nothing from it, and the doctest guards a symbol name nobody would add by chance. It still passes for any other duplicate entry point under a different name. The crate-root `//` comment at lines 12-15 explaining the absence of a `cfg_attr(test, allow(..))` block is the same pattern.

**Why it matters**: Process artifacts in `//!` docs go stale while still looking authoritative, and they add maintenance weight (a doctest kept alive for a name that no longer exists).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Crate docs describe what the crate provides (TokeiIngestor as the load entry point) without narrating the removed load_tokei function
- [x] #2 The compile_fail doctest for load_tokei and the matching tests.rs comment are removed, or replaced by a guard that does not depend on one historical symbol name

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.32 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
