---
id: TASK-1971
title: >-
  READ-5: the layout math assumes one terminal column per glyph for the
  config-supplied separator char and spinner frame, so a wide-glyph theme
  silently overflows every line
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
**File**: `crates/theme/src/configurable.rs:395-410` (`render_separator`), `crates/theme/src/configurable.rs:340-355` (`icon_prefix_parts`), `crates/theme/src/configurable.rs:126-129` (`running_template_overhead`)

**What**: three places convert a count of *glyphs* into a count of *columns* with an implicit 1:1 ratio, while the glyph itself comes from user TOML.

1. `render_separator` derives `sep_count` from a column budget and then pushes that many copies of `self.separator_char()`:

        let sep_count = space_for_sep.max(MIN_SEP_GLYPHS);
        let sep = self.separator_char();
        ...
        for _ in 0..dots_count { out.push(sep); }

   `separator_char` is a plain `char` deserialized from `[themes.*] separator_char`. Only its `len_utf8()` is consulted (for the `String::with_capacity` hint) — never its display width. A theme using a full-width separator (U+FF0E fullwidth full stop, U+3002 ideographic full stop, a box-drawing double glyph) produces a line exactly twice as wide as the budget it was computed from. Every downstream right-pad (`wrap_step_line`, `right_pad_with_border`) then measures with `visible_width` and saturates to 0, breaking the frame.

2. `icon_prefix_parts` reserves the spinner column as a literal:

        let (indent, spinner_cols) = if is_running { ("", 1usize) } else { (self.step_indent(), 0usize) };

   The spinner glyph actually rendered comes from `tick_chars`, a config string. The default sets are 1-column braille, but an emoji tick set (a common indicatif idiom) is 2 columns, and the running row's icon column then sits one cell right of every completed row.

3. `running_template_overhead` is a hand-maintained `usize` in the same config block that the theme author must keep consistent with `running_template` by eye. Nothing derives or validates it; `render_separator` subtracts it from the budget as fact.

**Why it matters**: this is the same class of defect as TASK-1844 (ops-core `EMOJI_COLS` hardcoded to 2), but on the config-driven side: the value is not a constant the project chose, it is an arbitrary glyph a user's `.ops.toml` supplies, and the crate's own `display_width` / `visible_width` helpers are right there but unused for it. A theme that renders fine for its author silently corrupts alignment for everyone whose terminal resolves the glyph at a different width.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 render_separator budgets the separator in display columns: it computes glyphs from space_for_sep divided by display_width(separator_char), so a wide separator char yields half as many glyphs and the same total width
- [ ] #2 the running-row spinner reserve is derived from the widest glyph in tick_chars via display_width rather than the literal 1usize
- [ ] #3 running_template_overhead is either derived from running_template or validated at ConfigurableTheme::new against it, with the mismatch surfaced rather than silently mis-budgeted
- [ ] #4 a test builds a theme with a full-width separator char and a two-column spinner glyph and asserts the rendered boxed step line still has visible_width equal to the border width
<!-- AC:END -->
