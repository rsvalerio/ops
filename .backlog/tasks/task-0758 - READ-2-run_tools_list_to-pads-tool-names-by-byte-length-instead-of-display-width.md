---
id: TASK-0758
title: >-
  READ-2: run_tools_list_to pads tool names by byte length instead of display
  width
status: To Do
assignee:
  - TASK-0828
created_date: '2026-05-01 05:54'
updated_date: '2026-05-01 06:18'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/tools_cmd.rs:29,46`

**What**: `let max_name_len = tools.iter().map(|t| t.name.len()).max()...` measures bytes, then `format!("{:width$}", tool.name, width = max_name_len)` pads in char count. Same defect class as TASK-0734 in help.rs render_grouped_sections — help-rendering side now uses display_width; tools-listing side still uses String::len.

**Why it matters**: Tool names are user-controlled (config [tools] keys) and can be non-ASCII. A name with a multibyte char will under- or over-count the column.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 run_tools_list_to measures column width via ops_core::output::display_width and pads with explicit space-fill (mirroring help.rs:render_grouped_sections)
- [ ] #2 Test inserts a tool name with width-2 characters plus an ASCII tool name and asserts the description column is aligned by display width
<!-- AC:END -->
