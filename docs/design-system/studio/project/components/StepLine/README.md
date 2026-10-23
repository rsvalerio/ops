# StepLine

One framed row per step: progress cell, icon, cyan label, a faint dot leader and a dim-yellow duration flush against the right bar.

**States:** pending `◇`, running (spinner in `ansi-magenta`, label in `ansi-cyan`, leader in `ansi-bright-black`, elapsed in a six-column field), succeeded `✓`, failed `✖`, skipped `⊘`.

**Anatomy (completed rows):** `│ ` · progress cell · `  ` · `  ` (`step_indent`) · icon · space · label · space · `.` run · space · duration · right pad · ` │`. The line is laid out at `columns − 7` (`cell-7`), the leader never shorter than three (`cell-3`).

**Running rows** render through `running_template = "│ ▓  {spinner:.magenta}{msg:.cyan} {elapsed:>6.yellow.dim} │"`; `running_template_overhead = 15`. The `>6` field (`cell-6`) keeps the right bar in the same column from `0.74s` to `99m59s`.

**Known quirk:** the running template has no `step_indent`, so a running label starts two columns left of the finished labels around it (`│ ▓  ⠁ cargo…` against `│ █    ✓ cargo…`). The specimen shows it as ops prints it; adding two spaces after `▓  ` in the template (and 2 to `running_template_overhead`) would align them.

**Colour:** label `ansi-cyan`, leader `ansi-bright-black`, duration `ansi-yellow-dim`; frame, cell and icon `term-fg`.

**Consumer supplies:** the step label, status and elapsed time.

Source: `render_slot` (`crates/theme/src/configurable.rs`), `wrap_step_line` (`crates/theme/src/configurable/boxed.rs`).
