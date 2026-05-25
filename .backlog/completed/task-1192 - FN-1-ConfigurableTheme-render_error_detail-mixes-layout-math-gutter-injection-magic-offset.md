---
id: TASK-1192
title: >-
  FN-1: ConfigurableTheme::render_error_detail mixes layout math, gutter
  injection, magic offset
status: Done
assignee:
  - TASK-1264
created_date: '2026-05-08 08:12'
updated_date: '2026-05-09 12:27'
labels:
  - code-review-rust
  - fn
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/theme/src/configurable.rs:162`

**What**: 35-line method handles: empty short-circuit, layout-kind branching, rail-width computation, gutter target arithmetic (BOX_STEP_RESERVE - BOX_FRAME_BARS + step_indent), extra_indent derivation via saturating subtraction, line-by-line inject_gutter_indent, and right_pad_with_border. Five distinct concerns interleaved with magic offset `+ 3` (line 182).

**Why it matters**: The `+ 3` is unexplained — it must encode the rail-prefix layout but no comment names it. Adding a new layout requires re-deriving the offset by reading three other functions.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The boxed-layout branch is extracted into a helper (e.g. boxed_error_indent_columns) whose return value is the documented gutter offset, and the magic + 3 is replaced with a named constant.
- [x] #2 A test pins the gutter alignment for at least two step_indent widths (0 and 2) so a future refactor does not silently mis-align the error glyph column.
<!-- AC:END -->
