---
id: TASK-0176
title: >-
  FN-3: StepLineTheme trait has 22 methods and mixes layout / styling / progress
  / summary / error concerns
status: To Do
assignee: []
created_date: '2026-04-22 21:25'
updated_date: '2026-04-23 06:45'
labels:
  - rust-code-review
  - FN
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: crates/theme/src/step_line_theme.rs:126-376

**What**: The StepLineTheme trait (the only required-to-implement method is status_icon, the rest have defaults) exposes 22 methods grouped into at least six concerns: left/indent padding (left_pad, left_pad_str, step_indent); icons (status_icon, icon_column_width); colors (header_color, label_color, separator_color, duration_color, summary_color); headers (plan_header_prefix, render_plan_header); separators (separator_char, render_summary_separator); running/progress (running_template, tick_chars, running_template_overhead); rendering (render, render_prefix, render_separator, format_elapsed, render_summary, summary_prefix); boxed layout (box_top_border, box_bottom_border, step_column_reserve, wrap_step_line); error detail (render_error_detail).

The doc block already acknowledges this ("This trait has 15 methods..." — stale; the real count is higher) and considers but rejects split traits / composition. The same doc block notes "Method count is stable (15 is acceptable)".

**Why it matters**: FN-3 / TRAIT-6 (ISP). A new theme implementer has to learn the whole surface even to tweak one glyph. Two viable refactors: (a) split into StepLineTheme + BoxedLayoutTheme + ErrorBlockTheme with blanket impls; (b) move the "look up value on ThemeConfig" defaults into a concrete Theme struct and make the trait hold only the 3-4 methods ConfigurableTheme actually customizes. At minimum, refresh the doc block and group the methods into the six concerns visually.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Update the doc block to reflect the real method count and group methods by concern
- [ ] #2 Consider splitting into sub-traits (StepLine + BoxedLayout + ErrorBlock) — if rejected, document why in the trait docs
<!-- AC:END -->
