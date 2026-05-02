---
id: TASK-0746
title: 'PERF-3: wrap_step_line allocates a fresh strip_ansi String per rendered row'
status: Done
assignee:
  - TASK-0828
created_date: '2026-05-01 05:52'
updated_date: '2026-05-02 07:36'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/theme/src/configurable.rs:201-223`

**What**: `wrap_step_line` calls `display_width(&strip_ansi(inner))`. `strip_ansi` walks every character and pushes into a new String. For boxed-layout runs with many steps this re-allocates per row.

**Why it matters**: Hot path on every step render. A `visible_width(s: &str) -> usize` helper that scans ANSI escapes inline avoids the allocation.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Introduce visible_width(s: &str) -> usize helper in theme/style.rs scanning ANSI escapes inline without allocating
- [x] #2 wrap_step_line and every other display_width(&strip_ansi(...)) site uses the new helper
- [x] #3 Test asserts the helper produces identical results to display_width(&strip_ansi(s)) across the existing strip_ansi corpus
<!-- AC:END -->
