---
id: TASK-0880
title: >-
  PERF-3: format_severity_section materializes Vec<AdvisoryRow> for width-only
  iteration
status: Triage
assignee: []
created_date: '2026-05-02 09:24'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/format.rs:203-211`

**What**: let rows: Vec<AdvisoryRow<_>> = entries.iter().map(&extract).collect(); then iterates rows three times to compute widths and write rows. The materialisation is unnecessary because extract is a pure projection - width passes can iterate entries and re-apply extract.

**Why it matters**: Minor allocation per render; the comment tag PERF-3 (TASK-0802) above the function explicitly documents the goal of "one allocation per render". This collect contradicts that intent and would amplify if cargo-deny output grows large.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Width passes call extract over entries directly without intermediate Vec
- [ ] #2 Render-time allocation count for a 100-entry advisory section drops by one Vec
- [ ] #3 format_severity_section keeps the borrow-from-T lifetime contract intact
<!-- AC:END -->
