---
id: TASK-0717
title: >-
  READ-6: run_about_coverage_with duplicates the lines_count==0 zero-coverage
  check from format_coverage_section
status: Done
assignee:
  - TASK-0738
created_date: '2026-04-30 05:30'
updated_date: '2026-04-30 18:39'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/coverage.rs:79`

**What**: `run_about_coverage_with` checks `if coverage.total.lines_count == 0 { writeln!(... "No coverage data available."); return; }`. `format_coverage_section` performs the same check (returns `vec![]`) immediately after. The caller therefore branches on a condition the renderer already encodes, and the two diverge in their fallback: the caller emits a literal message, the renderer returns empty.

**Why it matters**: READ-6 (consistent patterns) — two pieces of code own the "no coverage data" definition, so a future tweak (e.g. "treat lines_count < 1 as no data" or any new sentinel) has to be made in both places or the caller and renderer disagree. Either let `format_coverage_section` own the check and emit the message itself, or return a single signal (Option<Vec<String>>) so the caller cannot drift.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Single source of truth for the zero-coverage condition: either run_about_coverage_with or format_coverage_section
- [ ] #2 Pick one fallback shape (literal message vs empty lines) and document it
- [ ] #3 Update tests so the chosen contract is the only one pinned
<!-- AC:END -->
