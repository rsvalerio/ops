---
id: TASK-0189
title: 'TEST-11: format_coverage_section tests assert only substring presence'
status: To Do
assignee: []
created_date: '2026-04-22 21:25'
updated_date: '2026-04-23 06:45'
labels:
  - rust-code-review
  - TEST
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/coverage.rs:161-212`

**What**: format_coverage_section_with_units and format_coverage_section_skips_zero_unit assert substring contains only. Never pin line count, line ordering, exact total-line format. A refactor that reorders sections or changes total template would pass these tests.

**Why it matters**: TEST-11 — assert specific values, not just substring presence. For structured output the specific line format IS the contract.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Tests assert complete total line content including percentages and numeric formatting
- [ ] #2 Tests assert relative ordering of header, table rows, and total
<!-- AC:END -->
