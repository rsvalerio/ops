# BoxFrame

The enclosing frame of a studio run: plan title in the top border, one framed row per step, the summary in the bottom border.

**Anatomy:** `╭─` + title + `─` fill + `╮` · rows `│ <cell>  <line> │` · `╰─` + summary + `─` fill + `╯`. `left_pad = 0`, so the frame starts at column 0 and spans exactly `output.columns`.

**Live:** both borders are re-rendered as the run advances; the bottom border shows progress until it settles on the final summary.

**Reserve:** every row inside loses `BOX_STEP_RESERVE = 7` columns (`cell-7`) to the bars and the progress column, so step lines are laid out at `columns − 7`.

**Colour:** the frame glyphs are `term-fg`; only the titles inside the borders are coloured (header and summary).

**Consumer supplies:** the plan's command ids and the live counts; everything else is the theme.

Source: `box_top_border`, `box_bottom_border`, `wrap_step_line` (`crates/theme/src/configurable/boxed.rs`).
