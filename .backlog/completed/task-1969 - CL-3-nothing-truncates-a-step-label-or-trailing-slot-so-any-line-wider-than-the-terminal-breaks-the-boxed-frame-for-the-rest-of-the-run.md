---
id: TASK-1969
title: >-
  CL-3: nothing truncates a step label or trailing slot, so any line wider than
  the terminal breaks the boxed frame for the rest of the run
status: Done
assignee:
  - TASK-1987
created_date: '2026-08-27 15:54'
updated_date: '2026-08-28 19:30'
labels:
  - code-review-rust
  - idioms
dependencies: []
modified_files:
  - crates/theme/src/configurable.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/theme/src/configurable.rs:376-410` (`render_separator`), `crates/theme/src/configurable.rs:283-317` (`wrap_step_line`), `crates/theme/src/configurable.rs:487-495` (`wrap_box_content`), `crates/theme/src/configurable.rs:432-457` (`render_slot`)

**What**: the whole layout pipeline is written as "subtract the fixed costs, spend the remainder on the separator" — but the remainder is floored, never the content:

    let space_for_sep = line_budget.saturating_sub(fixed_inside);
    const MIN_SEP_GLYPHS: usize = 3;
    let sep_count = space_for_sep.max(MIN_SEP_GLYPHS);

When `fixed_inside` (prefix + trailing + 1) already exceeds `line_budget`, `space_for_sep` saturates to 0 and the floor then *adds* three more columns. `render_slot` concatenates prefix + separator + trailing unconditionally, so the returned line is `prefix_width + trailing_width + 4` columns wide regardless of `columns`. No caller clamps it, and there is no truncation helper anywhere in the crate (grep for 'truncate' in crates/theme/src returns nothing).

In flat layout that is a soft wrap. In boxed layout it is a broken frame:

    let inner_budget = outer.saturating_sub(frame_overhead);
    let right_pad = inner_budget.saturating_sub(inner_visible);
    ...
    out.push(' ');
    out.push('│');

`right_pad` saturates to 0 and the closing `│` is emitted anyway, past the terminal edge. The terminal wraps it onto the next physical row, so the right border of the box is one column short on that row and every subsequent row is visually offset — and under indicatif's multi-progress redraw the miscounted row height corrupts the rest of the frame. `wrap_box_content` has the identical shape for report detail lines, which are arbitrary-length strings the report producer supplies.

The precondition "the caller has already ensured the content fits" is nowhere stated in the type, the signature, or the docs — `render_slot` and `wrap_step_line` take `columns: u16` and give the impression of owning the budget.

**Why it matters**: a long command label (`cargo test --workspace --all-features -- --test-threads=1`) in an 80-column terminal, or any narrow terminal, is the ordinary case, not an adversarial one. The existing tests at `crates/theme/src/tests/edge_case_width.rs:60-79` render exactly this scenario and assert only `!line.is_empty()`, so the defect is invisible to the suite.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 render_slot clamps the assembled line to the column budget (truncating the label, with an ellipsis or a documented policy) rather than letting prefix plus MIN_SEP_GLYPHS plus trailing exceed columns
- [ ] #2 wrap_step_line and wrap_box_content truncate over-long inner content so the closing frame bar always lands at the same column as the borders
- [ ] #3 a test renders a step whose label is far wider than columns in boxed layout and asserts the wrapped line visible_width equals the width of box_top_border for the same columns
- [ ] #4 a test does the same for a report detail line passed through wrap_box_content
<!-- AC:END -->
