---
id: TASK-1522
title: >-
  DUP-1: deps format_upgrade_section duplicates row-write scaffolding across
  is_breaking arms
status: Done
assignee:
  - TASK-1645
created_date: '2026-05-19 07:32'
updated_date: '2026-05-25 17:41'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/format.rs:175-198`

**What**: The `is_breaking` arm and `else` arm in the row loop are near-identical `writeln!` calls — same name/old/arrow/new sequence — diverging only in the trailing `(latest …)` column. Changing the row shape (e.g., adding a `compatible` column) requires editing two writeln blocks in lockstep.

**Why it matters**: Bool-driven duplication is the FN-4 / DUP-1 shape that drifts: one branch was already updated for `is_breaking` while the other wasn't, and the next style change will hit the same trap.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Single writeln! call constructs the row, with the (latest …) suffix appended conditionally (e.g., via Option-formatted suffix or scratch string)
- [ ] #2 No behavioural change: existing format_upgrade_section_* tests pass
- [ ] #3 Width-pass code (latest_width) remains gated on is_breaking
<!-- AC:END -->
