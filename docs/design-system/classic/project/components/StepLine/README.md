# StepLine

One branch per step: `├── `, icon, label, a `─` rule, and the duration aligned to the right edge of the column budget.

**States:** pending `◇` (no duration, trailing space), running (ASCII spinner `|/-\` in `ansi-cyan`, elapsed in `term-dim`), succeeded `◆`, failed `✖`, skipped `⊘`.

**Anatomy (completed rows):** `cell-1` left pad · `├── ` (`cell-4`) · icon (one-cell column) · space · label · space · `─` run · space · duration. The rule is whatever is left of `output.columns`, never fewer than three (`cell-3`).

**Running rows** render through `running_template = "├── {spinner:.cyan}{msg} {elapsed:>6.dim}"`; `running_template_overhead = 12` reserves the template's columns.

**Truncation:** a label too long for the budget is cut with `…`; the duration is never cut first.

**Colour:** label, rule and duration are uncoloured in classic. State is the glyph alone: hollow ◇ waits, solid ◆ is done.

**Consumer supplies:** the step label (the command as run), status and elapsed time.

Source: `ConfigurableTheme::render` / `render_slot` (`crates/theme/src/configurable.rs`).
