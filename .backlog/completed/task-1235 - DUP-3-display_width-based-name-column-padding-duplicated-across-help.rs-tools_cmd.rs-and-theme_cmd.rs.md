---
id: TASK-1235
title: >-
  DUP-3: display_width-based name-column padding duplicated across help.rs,
  tools_cmd.rs, and theme_cmd.rs
status: Done
assignee:
  - TASK-1265
created_date: '2026-05-08 12:58'
updated_date: '2026-05-09 13:51'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/help.rs:144-180`, `crates/cli/src/tools_cmd.rs:34-71`, `crates/cli/src/theme_cmd.rs:72-100`

**What**: `render_grouped_sections`, `run_tools_list_to`, and `run_theme_list_to` each implement the same "compute max display_width over names, then pad each name with a manual space loop" pattern. Each call site comments that it mirrors one of the others; nothing extracts a shared helper.

**Why it matters**: Three-way duplicate with TASK-0758 / TASK-0734 / TASK-0936 cross-references guarantees drift the next time one site is tweaked. Test parity is also triplicated (`*_aligns_wide_char_names_by_display_width`).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Extract a shared helper in ops_core::output::pad_to_display_width
- [x] #2 Migrate all three call sites
- [x] #3 Route the existing alignment regressions through the shared helper
<!-- AC:END -->
