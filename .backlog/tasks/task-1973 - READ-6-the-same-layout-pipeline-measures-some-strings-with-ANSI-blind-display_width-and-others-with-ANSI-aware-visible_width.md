---
id: TASK-1973
title: >-
  READ-6: the same layout pipeline measures some strings with ANSI-blind
  display_width and others with ANSI-aware visible_width
status: To Do
assignee:
  - TASK-1987
created_date: '2026-08-27 15:54'
updated_date: '2026-08-28 14:10'
labels:
  - code-review-rust
  - readability
dependencies: []
modified_files:
  - crates/theme/src/configurable.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/theme/src/configurable.rs` — `display_width` at lines 168, 240-244, 341-342, 388-392, 570-576; `visible_width` at lines 300, 494, 527

**What**: the crate exposes two width functions with different semantics and the layout code mixes them within a single line's computation.

- `visible_width` (crates/theme/src/style/strip.rs:87-94) parses and skips ANSI escapes.
- `display_width` (crates/core/src/output.rs:9-11) is a bare `UnicodeWidthStr::width` — it has no ANSI grammar at all and scores an escape sequence's bytes as visible columns.

Both are applied to the same values in the same pipeline:

- `render_separator` measures the label prefix and the trailing slot with `display_width`.
- `wrap_step_line` measures the result of that same computation with `visible_width`.
- `build_horizontal_border` measures `title` with `display_width`, while `right_pad_with_border` measures the error-block line with `visible_width`.
- `icon_prefix_parts` and `icon_column_width` measure config-supplied icon glyphs with `display_width`.

Any string reaching the pipeline with an escape in it therefore gets two different widths depending on which half of the line is being computed, and the two halves disagree by exactly the escape's byte-length. The values concerned are not hypothetical: report row labels and results come from tool output, `plan_header_prefix` and the icon glyphs come from user TOML, and `report.title` / `footer_text()` come from the report producer. It also means the crate cannot honour its own guidance — the `visible_width` doc comment says "Hot-path callers should prefer this over the `display_width(&strip_ansi(...))` pair", yet the hottest path (`render_separator`, called once per rendered row) uses `display_width` directly.

**Why it matters**: consistency here is what makes the frame straight. Picking one measurement for the whole crate is also the precondition for fixing TASK-1969 (truncation) and the C1 handling in TASK-1967 — a truncation helper cannot be correct while two callers disagree on how wide the string it is truncating actually is.

Same class as TASK-1730 in ops-about (truncate_to_width char-sums widths while its siblings use display_width); this is the ops-theme instance and is not fixed by that task.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 one width function is used for every string measured in the render/layout pipeline in configurable.rs, and the choice is stated in a module-level comment
- [ ] #2 render_separator and build_horizontal_border measure with the ANSI-aware helper, so an escape-carrying label or title yields the same width the padding code assumes
- [ ] #3 a test renders a report row whose label already contains an SGR sequence and asserts the boxed line width matches the border width
<!-- AC:END -->
