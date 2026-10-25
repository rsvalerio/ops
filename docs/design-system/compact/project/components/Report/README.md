# Report

Command output shaped like a run, used by `ops deps`: a bold title, one row per check with a coloured result, and a footer.

**Rows** reuse the StepLine layout: icon from the `[report]` block (`✓` ok, `ℹ` info, `⚠` warning, `✘` error), section label, dot run, result string. Only the result is coloured: `ansi-green` ok, `term-dim` info, `ansi-yellow` warning, `ansi-red` error. The icon stays `term-fg`.

**Title:** `cell-bold` (`title_color = "bold"`), framed by blank rows.

**Footer:** `Done N checks`, plus ` · ` and the issue counts when there are any.

**Details:** per-row detail lines print verbatim under their row, with the same left pad and no dots.

The section titles in the preview are the real ones from `extensions-rust/deps/src/format.rs`; the results are illustrative.

Source: `ConfigurableTheme::render_report` (`crates/theme/src/configurable/report.rs`).
