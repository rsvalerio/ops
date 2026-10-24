# StepLine

One row per step: icon, label, a dot run, and the duration aligned to the right edge of the column budget.

**States:** pending `○` (no duration, trailing space), running (braille spinner in `ansi-cyan`, elapsed in `term-dim`), succeeded `✓`, failed `✗`, skipped `—`.

**Anatomy (completed rows):** `cell-1` left pad · `cell-2` indent · icon (one-cell column) · space · label · space · dots · space · duration. The dot run is whatever is left of `output.columns`, never fewer than three (`cell-3`).

**Running rows** drop the indent (the runner prepends `left_pad`) and render through `running_template = "  {spinner:.cyan}{msg} {elapsed:>6.dim}"`; `running_template_overhead = 10` reserves the template's columns.

**Truncation:** a label too long for the budget is cut with `…`; the duration is never cut first.

**Colour:** label, dots and duration are uncoloured in compact (`label_color`, `separator_color`, `duration_color` empty). State is the glyph alone.

**Consumer supplies:** the step label (the command as run), status and elapsed time.

Source: `ConfigurableTheme::render` / `render_slot` (`crates/theme/src/configurable.rs`).
