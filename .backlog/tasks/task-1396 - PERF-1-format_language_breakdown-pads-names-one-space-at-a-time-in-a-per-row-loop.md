---
id: TASK-1396
title: >-
  PERF-1: format_language_breakdown pads names one space at a time in a per-row
  loop
status: To Do
assignee:
  - TASK-1458
created_date: '2026-05-13 18:06'
updated_date: '2026-05-13 19:09'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/project_identity/format.rs:137-140`

**What**: For each language row, padding is built by `for _ in 0..pad { padded_name.push(' '); }`. A bulk approach (`padded_name.reserve(pad); padded_name.extend(std::iter::repeat(' ').take(pad))` or formatting with `{:<width$}`) replaces N push calls per row with a single reserve+extend.

**Why it matters**: Minor allocation/branch overhead but trivially avoidable with the standard idiom; aligns with the broader display-formatting helpers already in this crate.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Per-character push loop replaced with a bulk pad (format width specifier or extend) producing identical output
- [ ] #2 project_identity format tests pass unchanged
<!-- AC:END -->
