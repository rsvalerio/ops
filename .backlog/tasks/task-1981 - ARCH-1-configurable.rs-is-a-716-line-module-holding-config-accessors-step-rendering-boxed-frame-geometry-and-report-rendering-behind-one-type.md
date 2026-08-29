---
id: TASK-1981
title: >-
  ARCH-1: configurable.rs is a 716-line module holding config accessors, step
  rendering, boxed-frame geometry and report rendering behind one type
status: Done
assignee:
  - TASK-1987
created_date: '2026-08-27 15:56'
updated_date: '2026-08-28 19:29'
labels:
  - code-review-rust
  - architecture
dependencies: []
modified_files:
  - crates/theme/src/configurable.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/theme/src/configurable.rs` (716 lines, 35+ public items on one impl block)

**What**: the file has grown past the ARCH-1 module red flag (>500 lines, mixed unrelated concerns). Four distinct responsibilities share it:

1. **Config passthrough** — lines 85-160: twenty one- or two-line accessors (`left_pad`, `status_icon`, `separator_char`, `step_indent`, `summary_prefix`, `running_template`, `tick_chars`, `header_color`, `label_color`, `separator_color`, `duration_color`, `summary_color`, `plan_header_prefix`, …) that only forward to `self.config`. They exist because the fields went private in TASK-0748; they carry no logic and dominate the file's public surface.
2. **Flat step rendering** — `step_prefix_parts`, `icon_prefix_parts`, `render_prefix`, `render_separator`, `render`, `render_slot`, `render_summary`, `render_summary_text`.
3. **Boxed-frame geometry** — the three `BOX_*` constants, `boxed_error_indent_columns`, `step_column_reserve`, `box_top_border`, `box_bottom_border`, `wrap_step_line`, `wrap_box_content`, plus the free functions `inject_gutter_indent`, `right_pad_with_border`, `build_horizontal_border` and the `BorderArgs` struct at the bottom of the file.
4. **Report rendering** — `render_report`, `render_report_boxed`, `report_slot`, `report_icon`, `report_prefix` and the five `report_*_prefix` fields.

The crate is already organised this way elsewhere and the seams are visible: `render.rs` holds the error block, `step_line_theme.rs` holds the shared value types, `style/` was split into `sgr` and `strip` under ARCH-1/TASK-0881 for exactly this reason. Concerns 3 and 4 have no dependency on each other and only touch concern 2 through `render_slot`.

**Why it matters**: the boxed geometry is the part of this crate most in need of careful reading — TASK-1969, TASK-1971 and TASK-1973 all land in it — and it is currently interleaved with twenty trivial getters and the report path. Splitting it (for example `configurable/boxed.rs` and `configurable/report.rs`, or moving the free border helpers into their own module) makes the column arithmetic reviewable on its own.

Filed Low: no defect today, but the file is the natural home for the fixes queued against it and will grow further.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 the boxed-frame geometry (BOX_ constants, border builders, wrap helpers, BorderArgs) lives in its own module
- [ ] #2 the report rendering path lives in its own module
- [ ] #3 configurable.rs retains the ConfigurableTheme type, its constructor and the shared render_slot seam, and is under the 500-line ARCH-1 threshold
- [ ] #4 no public API changes: every item currently reachable from lib.rs stays reachable at the same path
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
configurable.rs split into configurable/{boxed,report,config_access}.rs; parent is 451 lines (under the 500-line ARCH-1 threshold). No public API change: every item stays a method on ConfigurableTheme (or a private free fn) at the same path.
<!-- SECTION:NOTES:END -->
