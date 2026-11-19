---
id: TASK-2715
title: 'TEST-5: strip_ansi_preserving_raw has no tests in ops-theme'
status: To Do
assignee: []
created_date: '2026-11-18 18:32'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - test-quality
dependencies: []
parent_task_id: 'TASK-2845'
modified_files:
  - crates/theme/src/style/strip.rs
  - crates/theme/src/style.rs
priority: low
ordinal: 1000
dedup_key: 'review-2667.06-6'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/theme/src/style/strip.rs:310` (`strip_ansi_preserving_raw`)

**What**: `strip_ansi_preserving_raw` is a public function with a distinct, documented policy (complete escapes removed; Raw runs, control characters and tab kept verbatim; `Cow::Borrowed` fast path when no introducer is present). No test in `crates/theme` (unit, `tests/public_api`, or the proptest in `src/style.rs`) calls it; its only coverage is indirect, through `ops-cargo-update`'s parser tests in another crate.

**Why it matters**: The module docs state the iterator is the workspace's only ANSI grammar and that each policy function decides the fate of Raw pieces and controls differently. A change to the grammar or to this policy in `ops-theme` would pass this crate's suite and surface (if at all) as a cargo-update parse regression, far from the cause.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 ops-theme tests assert strip_ansi_preserving_raw removes complete CSI/OSC/C1 sequences, keeps truncated or stray escape bytes and control characters verbatim, and keeps tab
- [ ] #2 A test asserts the no-introducer input returns Cow::Borrowed
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.06 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
