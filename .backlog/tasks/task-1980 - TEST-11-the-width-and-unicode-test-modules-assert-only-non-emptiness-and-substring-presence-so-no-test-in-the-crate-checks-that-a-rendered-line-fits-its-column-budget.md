---
id: TASK-1980
title: >-
  TEST-11: the width and unicode test modules assert only non-emptiness and
  substring presence, so no test in the crate checks that a rendered line fits
  its column budget
status: Triage
assignee: []
created_date: '2026-08-27 15:56'
labels:
  - code-review-rust
  - test-quality
dependencies: []
modified_files:
  - crates/theme/src/tests/edge_case_width.rs
  - crates/theme/src/tests/unicode.rs
  - crates/theme/src/tests/render_basics.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/theme/src/tests/edge_case_width.rs:8-79`, `crates/theme/src/tests/unicode.rs:5-53`, `crates/theme/src/tests/render_basics.rs:211-222`

**What**: the two modules whose stated purpose is width behaviour never assert a width. The assertions are:

- `render_with_zero_columns_does_not_panic`, `render_with_one_column_does_not_panic`, `render_with_two_columns_does_not_panic`, `render_with_very_small_columns_handles_gracefully`, `render_pending_with_zero_columns`, `render_failed_with_minimal_columns`, `render_label_longer_than_columns` — all `assert!(!line.is_empty())`.
- `render_handles_unicode_labels`, `render_handles_emoji_in_label`, `render_handles_mixed_width_unicode`, `render_handles_very_long_unicode_label`, `render_handles_right_to_left_text` — all `assert!(line.contains(<the input>))`, which is true for any function that echoes its argument.
- `classic_theme_very_small_columns` and `compact_theme_very_small_columns` in render_basics.rs — `assert!(line.contains("cmd"))`.
- `render_separator_label_longer_than_columns` asserts `sep.len() <= 200`, a byte-length bound with no relationship to the contract; the real contract (the separator never pushes the line past `columns`) is not asserted.

Two of these render exactly the case that is broken today. `render_handles_very_long_unicode_label` renders `"构建".repeat(50)` — 200 display columns — at `columns = 80`. `render_label_longer_than_columns` renders a 69-character label at `columns = 20`. Both pass while the produced line is three to ten times the requested width (see TASK-1969), because neither looks at the width.

The crate demonstrably knows how to write the assertion — `boxed_layout.rs` and `render_report.rs` assert `display_width(&strip_ansi(line)) == columns` in five places. That measurement is simply absent from the modules that feed the layout the hard inputs.

**Why it matters**: this is the coverage half of TASK-1969 and TASK-1971. Fixing the truncation and the glyph-width budgeting without strengthening these tests leaves the regression surface exactly as wide as it is now: a future change can re-break line width and this suite stays green.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 every test in edge_case_width.rs and unicode.rs that renders at a given columns value asserts the resulting visible width against that budget, in addition to any non-emptiness or containment check it already makes
- [ ] #2 render_handles_very_long_unicode_label and render_label_longer_than_columns assert the truncation policy chosen in TASK-1969 rather than only that the input echoes back
- [ ] #3 render_separator_label_longer_than_columns replaces the arbitrary sep.len() bound of 200 bytes with an assertion in display columns tied to the columns argument
- [ ] #4 at least one test covers a CJK or emoji label at a narrow width in boxed layout and pins the line width to the border width
<!-- AC:END -->
