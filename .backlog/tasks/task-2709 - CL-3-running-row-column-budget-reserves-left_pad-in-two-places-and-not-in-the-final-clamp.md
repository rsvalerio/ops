---
id: TASK-2709
title: 'CL-3: running-row column budget reserves left_pad in two places and not in the final clamp'
status: To Do
assignee: []
created_date: '2026-11-18 18:32'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - cognitive-load
dependencies: []
parent_task_id: 'TASK-2845'
modified_files:
  - crates/theme/src/configurable.rs
priority: low
ordinal: 1000
dedup_key: 'review-2667.06-3'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/theme/src/configurable.rs:382` (`ConfigurableTheme::render_slot_gated`), with `label_budget` (`configurable.rs:421`) and `render_separator` (`configurable.rs:290`)

**What**: For a running row (`is_running == true`) `render_slot_gated` emits no left pad (`let pad = if slot.is_running { "" } ...`), yet `label_budget` and `render_separator` both subtract `self.left_pad()` together with `running_template_overhead`, while the final belt-and-braces clamp uses `budget.saturating_sub(template_overhead)` with no `left_pad`. The three computations of "what this running row may spend" disagree, and nothing says which is intended: with the built-in `classic` theme (`running_template_overhead = 12`, which already covers `├── `, spinner, space and the 6-column elapsed field, `left_pad = 1`) a running row ends one column short of the terminal width. The overhead expression `if is_running { self.running_template_overhead() } else { 0 }` is itself recomputed in all three places.

Related doc drift in the same arithmetic: `render_separator`'s doc says "a floor of 3 so the separator is always at least three glyphs wide", but `sep_cols` counts the leading space, so the minimum is two separator glyphs (`MIN_SEP_GLYPHS` is documented as columns, which is the accurate reading).

**Why it matters**: The module docs make "no rendered line may exceed the column budget it was given" the core invariant, and this is the arithmetic that enforces it. An undocumented, inconsistent reservation is an implicit assumption a future change to either function can silently break (or that already wastes a column on every running row).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The running-row reservation (template overhead plus or minus left_pad) is computed by one helper used by label_budget, render_separator and the final clamp in render_slot_gated
- [ ] #2 Whether a running row reserves left_pad is decided and documented, with a test pinning the rendered running-row width for a built-in theme against columns
- [ ] #3 render_separator's doc states the actual minimum separator run (columns including the leading space, or glyphs) consistently with MIN_SEP_GLYPHS
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.06 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
