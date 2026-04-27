---
id: TASK-0404
title: 'ERR-1: parse_upgrade_table silently drops short rows without diagnostic'
status: To Do
assignee:
  - TASK-0421
created_date: '2026-04-26 09:52'
updated_date: '2026-04-26 10:10'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/parse.rs:45-90`

**What**: `parse_upgrade_table` skips lines whose whitespace-split has fewer than 5 fields with no logging, no count, and no fallback. Combined with the existing whitespace-split fragility (TASK-0383), a schema change in `cargo upgrade` (added column, removed column, wrapped row) produces an empty/partial report and no signal that anything was lost.

**Why it matters**: The deps command will say "no upgrades available" when in fact the parser silently dropped every row. The downstream caller (`run_deps`) then exits 0 because `has_issues` sees nothing. Operators have no way to detect drift versus genuine clean state.

**Suggested**: emit `tracing::debug!` for skipped rows with a sample, or count them and surface in the parsed result for diagnostic purposes.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 parse_upgrade_table emits a tracing::debug or returns a skipped-row count when rows are dropped
- [ ] #2 Test asserts that malformed input emits visible diagnostic without panicking
<!-- AC:END -->
