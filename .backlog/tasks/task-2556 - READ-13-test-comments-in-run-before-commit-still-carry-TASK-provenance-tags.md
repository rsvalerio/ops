---
id: TASK-2556
title: 'READ-13: test comments in run-before-commit still carry TASK provenance tags'
status: To Do
assignee: []
created_date: '2026-10-10 15:40'
updated_date: '2026-10-10 21:12'
labels:
  - code-review-rust
  - readability
dependencies: []
parent_task_id: 'TASK-2624'
modified_files:
  - extensions/run-before-commit/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions/run-before-commit/src/lib.rs:mod tests'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/run-before-commit/src/lib.rs:144`, `:239`, `:280`, `:336`, `:368`, `:675`

**What**: Six `//` comments inside `mod tests` open with rule/TASK provenance tags: "TEST-18 / TASK-2119" (:144, the serial-attr banner), "TEST-15 / TASK-2113" (:239 and :280, the empty-PATH tempdir rationale), "TEST-5 / TASK-2133" (:336 and :368, the returned-path assertion rationale), and "TEST-15 / TASK-1913 AC#3" (:675, the hang-detector bound). TASK-2136 (Done) cleaned the `///`/`//!` blocks and its AC permitted tags "moved to a comment inside the body"; the current standard (TASK-2519, TASK-2522, and the October commits e5b5443f / 1d9bc644 / 83789471 stripping task tags from other crates' tests) flags body and test comments too.

**Why it matters**: READ-13: rule/TASK tags are review-process narration — they point at a backlog entry a reader of the code cannot resolve, and they go stale while looking authoritative. The technical rationale around each tag (why every spawning test needs `#[serial_test::serial]`, why an empty tempdir on PATH beats /usr/bin:/bin, why the returned hook path is asserted, why the timeout test has no wall-clock budget) is enduring and must be kept; only the tag prefix goes.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 No comment in the tests module opens with a RULE-ID / TASK-XXXX tag
- [ ] #2 The enduring rationale at each of the six sites survives the tag removal (serial-attr justification, PATH isolation, returned-path assertion, hang-detector bound)
<!-- AC:END -->
