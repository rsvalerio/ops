---
id: TASK-1975
title: >-
  PERF-3: icon_column_width re-measures every status icon on every rendered row,
  in a struct built specifically to precompute per-render work
status: Triage
assignee: []
created_date: '2026-08-27 15:55'
labels:
  - code-review-rust
  - performance
dependencies: []
modified_files:
  - crates/theme/src/configurable.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/theme/src/configurable.rs:165-172` (`icon_column_width`), called from `crates/theme/src/configurable.rs:341` (`icon_prefix_parts`) and `crates/theme/src/configurable.rs:199` (`render_error_detail`)

**What**:

    pub fn icon_column_width(&self) -> usize {
        ALL_STATUSES
            .iter()
            .map(|s| display_width(self.status_icon(*s)))
            .max()
            .unwrap_or(0)
    }

Every call walks all statuses, does a `ThemeConfig::status_icon` match per status, and runs a full `UnicodeWidthStr::width` scan over each glyph. `icon_prefix_parts` calls it once per row, and `icon_prefix_parts` is on the path of `render` (every step line, re-rendered on every progress tick), `render_prefix`, `render_slot`, and every report row. `boxed_error_indent_columns` and `render_error_detail` call it again per error block.

The value depends only on `self.config`, which is moved into `ConfigurableTheme` at construction and never mutated — the struct is `#[non_exhaustive]` with private fields and no setter. It is therefore constant for the theme's lifetime.

This is the one derived value the constructor missed. `ConfigurableTheme::new` already precomputes eleven SGR prefixes (TASK-0747) and `left_pad_str` (TASK-1035) for exactly this reason, and the crate has a PERF-3 history of removing per-render allocations (TASK-1130, TASK-0746). Storing `icon_column_width` as a field is the same one-line change those tasks made.

**Why it matters**: small per call, but it is multiplied by rows times redraw ticks, and it is inconsistent with the documented design of the type — a reader looking at the precomputed-prefix block reasonably assumes all derived layout constants live there.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 icon_column_width is computed once in ConfigurableTheme::new and stored as a private field alongside left_pad_str, with the accessor returning the stored value
- [ ] #2 the accessor stays public and keeps its current signature so existing callers are unchanged
- [ ] #3 a test asserts the accessor still returns the widest ALL_STATUSES glyph width for a theme with a multi-column custom icon
<!-- AC:END -->
