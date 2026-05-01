---
id: TASK-0794
title: >-
  FN-1: parse_upgrade_table is 98 lines mixing column-offset state, header
  detection, separator parsing, and per-row slicing
status: Triage
assignee: []
created_date: '2026-05-01 05:59'
labels:
  - code-review-rust
  - structure
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/parse.rs:49-147`

**What**: One function tracks the optional columns state across header / separator / data rows, computes column offsets via a closure, handles five required-field destructuring with mass-Some(...) matching, then folds the trailing-note absorption logic.

**Why it matters**: Three responsibilities in one body: parser state machine, byte-offset slicing, entry construction. TASK-0383, TASK-0404, TASK-0609 each touched a different concern in the same body — a sign the function is doing too much.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extract the row-parser (offsets → Option<UpgradeEntry>) into a free function
- [ ] #2 Keep parse_upgrade_table as the line iterator + state transitions only
- [ ] #3 Function <=50 lines after refactor; each helper handles one concern
<!-- AC:END -->
