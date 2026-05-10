---
id: TASK-0351
title: 'READ-5: render_separator mixes byte length with display width'
status: Done
assignee:
  - TASK-0418
created_date: '2026-04-26 09:35'
updated_date: '2026-04-27 10:25'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/theme/src/step_line_theme.rs:392`

**What**: fixed_inside = prefix_width + duration_str.len() + leading_space adds display_width(prefix) (display columns) to duration_str.len() (UTF-8 bytes). format_elapsed is overridable on the trait and the invariant is nowhere enforced.

**Why it matters**: A custom theme returning a non-ASCII duration ("0,35s", "⏱ 1.2s") will misalign the separator and overflow the boxed line budget.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace duration_str.len() with display_width(duration_str) (already imported)
- [ ] #2 Add a test using a theme override whose format_elapsed returns a multi-byte/wide string
<!-- AC:END -->
