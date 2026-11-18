---
id: TASK-2708
title: 'FN-1: unit_from_use_dir mixes classification, logging, I/O and description shaping'
status: Done
assignee: []
created_date: '2026-11-18 18:32'
updated_date: '2026-11-18 22:20'
labels:
  - code-review-rust
  - FN
dependencies: []
parent_task_id: 'TASK-2846'
modified_files:
  - extensions-go/about/src/modules.rs
priority: low
ordinal: 1000
dedup_key: 'review-2667.07-2'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-go/about/src/modules.rs:79`

**What**: `unit_from_use_dir` (modules.rs:79-150, ~72 lines) does four jobs its own doc comment enumerates: path normalisation, out-of-tree classification with its diagnostic, the per-module `go.mod` read, and description shaping. The classification is three overlapping booleans — `out_of_tree_via_components` (RootDir/Prefix/ParentDir), `out_of_tree_via_string` (first `/`/`\` segment is `..`, which duplicates the ParentDir arm on Unix), and `has_embedded_traversal` — plus a fourth, `is_absolute_directive`, re-deriving part of the first from `first_component`. Which combination skips the read, which warns, and which marks the description is spread across lines 86-145.

**Why it matters**: This is the crate's path-traversal policy (SEC-14 adjacent); a reader has to reconstruct a 3x2 truth table from scattered booleans to audit it, and the next edit to one predicate can silently desynchronise the warn/skip/describe decisions.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Directive classification is extracted into one function returning an enum (e.g. InTree / OutOfTreeReadable / OutOfTreeUnreadable) that drives the go.mod read, the single warn, and the description suffix
- [x] #2 unit_from_use_dir is ≤50 lines and operates at one abstraction level
- [x] #3 Existing modules.rs tests (absolute, embedded .., leading .., ..staging) pass unchanged

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.07 (code-review-rust run, 2026-11-18).

DirectivePlacement enum (InTree / OutOfTreeReadable / OutOfTreeAbsolute / EmbeddedTraversal) with classify, reads_go_mod, warn and describe; unit_from_use_dir is now ~17 lines. Absolute is split from embedded traversal to keep their distinct warn messages.

<!-- SECTION:NOTES:END -->
