---
id: TASK-1516
title: >-
  FN-1: deps interpret_upgrade_output Some(0) arm packs header-drift,
  row-shape-drift, and parse into a 73-line function
status: To Do
assignee:
  - TASK-1574
created_date: '2026-05-19 07:27'
updated_date: '2026-05-19 16:45'
labels:
  - code-review-rust
  - fn
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/parse/upgrade.rs:39-111` (`interpret_upgrade_output`)

**What**: `interpret_upgrade_output` is 73 lines (39-111). The `Some(0)` arm alone covers ~50 lines and embeds three separate concerns:

- the header-drift bail (TASK-1074, lines 56-66),
- the row-shape-drift bail (TASK-1202, lines 72-89),
- the happy-path return (line 90).

Each branch logs a distinct `tracing::warn!` plus its own `anyhow::bail!`. The arm reads like a state machine inlined into the match; extracting the two drift checks into named helpers (e.g. `check_header_drift`, `check_row_shape_drift`) returning `anyhow::Result<()>` would make the `Some(0)` arm three lines and keep the diagnostics auditable side-by-side.

**Why it matters**: FN-1 (function > 50 lines). Compound: the two drift checks share the same `tracing::warn!` + `anyhow::bail!` shape; extracting them also enables direct unit tests of each predicate without driving them through the full `interpret_upgrade_output` entry point.

<!-- scan confidence: function length verified — 73 lines per `awk '/^pub fn interpret_upgrade_output/,/^}/' | wc -l` -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 interpret_upgrade_output Some(0) arm reduced below 50 lines by extracting the header-drift and row-shape-drift checks into named helpers
- [ ] #2 Each extracted helper returns anyhow::Result<()> and preserves its existing tracing::warn! call (TASK-1074, TASK-1202) verbatim
- [ ] #3 interpret_upgrade_output_bails_on_unrecognised_header_with_separator and interpret_upgrade_output_bails_on_row_shape_drift continue to pass unchanged
<!-- AC:END -->
