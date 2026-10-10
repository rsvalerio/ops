---
id: TASK-2471
title: 'FN-1: parse_update_output is ~65 code lines, over the 50-line threshold'
status: Done
assignee: []
created_date: '2026-10-10 15:28'
updated_date: '2026-10-10 21:38'
labels:
  - code-review-rust
  - function-structure
dependencies: []
parent_task_id: 'TASK-2615'
modified_files:
  - extensions-rust/cargo-update/src/lib.rs
priority: medium
ordinal: 1000
dedup_key: 'FN-1:extensions-rust/cargo-update/src/lib.rs:parse_update_output'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-update/src/lib.rs:242`

**What**: `parse_update_output` spans lines 242-327 (86 raw lines; ~65 excluding interior comments), exceeding the FN-1 threshold of 50 lines. The function mixes three jobs in one body: noise-line filtering (the empty/`Locking`/`Unchanged`/`warning:`/`note:` cascade plus the `is_index_progress_line` guard), per-outcome handling of `ActionLineOutcome` (count increment, `tracing::warn!` on Rejected and on drift-suspect NoMatch), and result assembly.

**Why it matters**: FN-1 asks each function to operate at a single abstraction level. The noise-filter cascade and the outcome-handling match are each extractable at one level below the loop, which would leave `parse_update_output` as a ~20-line orchestration loop. The parser already extracts `parse_action_line` and its helpers, so this is the one remaining mixed-level body in the crate.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 parse_update_output body is at or under 50 lines, or carries a comment documenting why the parser-loop shape justifies an exception
- [x] #2 Noise filtering and ActionLineOutcome handling each live in a named helper at a single abstraction level
- [x] #3 All existing parser, property, and warn-capture tests pass unchanged

<!-- AC:END -->
