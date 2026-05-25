---
id: TASK-1492
title: >-
  PATTERN-1: parse_upgrade_table_inner body_lines counter includes pre-header
  preamble, causing false-positive row-shape-drift bail
status: Done
assignee:
  - TASK-1645
created_date: '2026-05-18 17:28'
updated_date: '2026-05-25 17:40'
labels:
  - code-review-rust
  - correctness
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/parse/upgrade.rs:143-214` and `:67-89`

**What**: `parse_upgrade_table_inner` increments `body_lines` for every non-empty, non-header, non-separator line — including lines that appear *before* the recognised header / `====` separator. The downstream gate in `interpret_upgrade_output` then trips on:

```
diag.saw_recognised_header
    && diag.saw_separator
    && diag.body_lines > 0
    && diag.entries_emitted == 0
```

If cargo-edit ever prints any preamble (banner, "Updating crates.io index", warning text) followed by a recognised header + separator + zero data rows, `body_lines` is non-zero (preamble), `entries_emitted` is zero (no rows), and the run bails with the TASK-1202 "row-shape drift" error even though no body row was actually attempted. The existing test `parse_upgrade_table_no_data_rows` only covers the no-preamble shape and does not catch this.

`body_lines` should count only lines observed *after* the separator (i.e. real candidate body rows that `parse_upgrade_row` could be asked to consume); pre-header / pre-separator lines must not feed the drift heuristic.

**Why it matters**: The deps gate is supposed to fail loudly on cargo-edit format drift, not on cargo-edit emitting a normal preamble with no available upgrades. A false-positive bail flips a clean supply-chain check into a CI failure with a misleading "row-shape drift" message, eroding trust in the gate.

<!-- scan confidence: candidates to inspect -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 body_lines no longer counts lines observed before saw_separator is set
- [ ] #2 Regression test: preamble + recognised header + separator + zero body rows returns Ok([]) and does NOT bail with row-shape-drift
- [ ] #3 Existing TASK-1202 row-shape-drift behaviour remains green (real body rows that fail the 5-column shape still bail)
<!-- AC:END -->
