# StepLine

One row per step: `▸`, an emoji status, a cyan label, a recessive ━ rule, and a dim-yellow duration at the right edge.

**States:** pending `⏸`, running (braille spinner in `ansi-magenta`, label in `ansi-cyan`), succeeded `✅`, failed `💥`, skipped `⏭`.

**Anatomy (completed rows):** `cell-1` left pad · `  ▸ ` (`cell-4`) · icon column (`cell-2`: emoji are two cells; `⏸` and `⏭` get one pad space) · space · label · space · ━ run · space · duration. The rule fills what is left of `output.columns`, never fewer than three (`cell-3`).

**Colour:** label `ansi-cyan` (`label_color`), rule `ansi-bright-black` (`separator_color`, applied to the run and its surrounding spaces), duration `ansi-yellow-dim` (`duration_color`). The icon and `▸` are uncoloured.

**Running rows** render through `running_template = "  ▸ {spinner:.magenta}{msg:.cyan} {elapsed:>6.yellow.dim}"`; `running_template_overhead = 12`. The template colours the message cyan, but the label and the rule carry their own colour sequences, each ending in a reset, so the rule stays `ansi-bright-black` and the elapsed field (`ansi-yellow-dim`, right-aligned in six columns) ends in the same column as a finished duration.

**Truncation:** a long label is cut with `…`; the duration is never cut first.

**Consumer supplies:** the step label, status and elapsed time.

Source: `ConfigurableTheme::render` / `render_slot` (`crates/theme/src/configurable.rs`).
