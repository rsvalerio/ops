---
id: TASK-2161
title: 'FN-1: scan_tokei is 88 statements spanning root validation, walking, four skip policies, tokei dispatch and shortfall accounting'
status: To Do
assignee: []
created_date: '2026-09-08 07:05'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-rust
  - complexity
dependencies: []
parent_task_id: 'TASK-2246'
modified_files:
  - extensions/tokei/src/lib.rs
priority: low
ordinal: 74000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/tokei/src/lib.rs:214-326`

**What**: `scan_tokei` is 113 lines, 88 of them non-comment and non-blank — well past the 50-line guideline. It carries five distinct responsibilities in one body:

1. scan-root validation (`metadata` + `ensure!` the root is a directory),
2. walker construction and the per-entry deadline cancellation check,
3. candidate selection with four independent skip policies (walk error, non-file, unrecognised language, unreadable metadata),
4. the two bound checks (`file_bytes`, `files`) with their warn-and-count bookkeeping,
5. the empty-candidate guard, the `Languages::get_statistics` dispatch, and the `candidates.len() - records.len()` shortfall inference.

Each of the three `skipped_*` counters is mutated from more than one branch, which is what makes the body hard to read as a whole: the accounting is spread across the loop rather than stated once.

**Why it matters**: the counters are the crate's only signal that a statistic is short rather than correct (`TokeiScan`'s whole purpose), and verifying they are maintained correctly currently requires holding all five concerns in view at once. Two of the pending findings on this file — the missing record sort and the double walk — both land inside this function, and a smaller `collect_candidates(&Path, ScanLimits, Option<&Deadline>) -> anyhow::Result<(Vec<PathBuf>, Skips)>` split would let each be changed in isolation.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The candidate-collection walk (root validation through the file cap) is extracted into its own function, leaving scan_tokei as the composition of validate, collect, count, account
- [ ] #2 Each extracted function is under 50 non-comment lines
- [ ] #3 The skip counters are returned as one value from the walk rather than three separately mutated locals
- [ ] #4 No behavioural change: the existing scan-bound, error-root and unreadable-file tests pass unmodified
<!-- AC:END -->
