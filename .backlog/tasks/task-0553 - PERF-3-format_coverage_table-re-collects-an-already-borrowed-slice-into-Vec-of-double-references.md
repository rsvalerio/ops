---
id: TASK-0553
title: >-
  PERF-3: format_coverage_table re-collects an already-borrowed slice into Vec
  of double references
status: Triage
assignee: []
created_date: '2026-04-29 05:02'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/coverage.rs:126`

**What**: format_coverage_table receives a slice of &UnitCoverage then does `let mut sorted: Vec<&&UnitCoverage> = units.iter().collect()` before sorting — the outer reference is redundant (the slice elements are already references) and the function then iterates &sorted producing triple-references. Sorting could operate on Vec<&UnitCoverage> (or sort indices) without the extra layer.

**Why it matters**: Two unnecessary indirections; signature reads like a double-ref was intended for borrow-elision when it is an artefact.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 sorted is Vec<&UnitCoverage> (or function takes &mut [&UnitCoverage] and sorts in place)
- [ ] #2 No behaviour change in the rendered table
<!-- AC:END -->
