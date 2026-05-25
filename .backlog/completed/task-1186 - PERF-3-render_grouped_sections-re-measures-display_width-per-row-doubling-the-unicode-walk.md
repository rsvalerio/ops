---
id: TASK-1186
title: >-
  PERF-3: render_grouped_sections re-measures display_width per row, doubling
  the unicode walk
status: Done
assignee:
  - TASK-1263
created_date: '2026-05-08 08:11'
updated_date: '2026-05-09 11:10'
labels:
  - code-review-rust
  - perf
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/help.rs:152`

**What**: For N command entries the code calls `display_width(&e.name)` once for the max computation and again inside the per-entry loop (`name_cols = display_width(&entry.name)`), doubling the unicode-width walk per row.

**Why it matters**: Help is rendered on every `ops --help` and on every `ops` invocation with no subcommand. display_width is O(chars) and the second walk is pure waste.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Each entry's display_width is computed exactly once per render_grouped_sections call.
- [x] #2 A test pins that calls into display_width equal entries.len(), not 2 * entries.len().
<!-- AC:END -->
