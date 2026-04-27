---
id: TASK-0354
title: 'DUP-5: render and render_prefix duplicate icon/indent/padding logic'
status: Done
assignee:
  - TASK-0414
created_date: '2026-04-26 09:35'
updated_date: '2026-04-26 10:24'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/theme/src/step_line_theme.rs:306`

**What**: render (306-348) recomputes icon, icon_width, max_icon_width, indent, spinner_cols, icon_pad exactly as render_prefix (351-362) does. The two implementations must stay byte-identical for display_width(plain_prefix) (used downstream by render_separator) to remain correct.

**Why it matters**: Any future change to icon/indent rules must be made in two places; if they drift, the separator length silently miscomputes.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Introduce a private helper (or small struct) returning indent + icon + pad + label segments; call it from both render and render_prefix
- [ ] #2 A test using a theme with a multi-character icon asserts display_width(plain_prefix) equals sum of helper component widths
<!-- AC:END -->
