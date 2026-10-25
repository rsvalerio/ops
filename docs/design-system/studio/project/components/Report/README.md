# Report

`ops deps` output in the studio frame: report title in the top border, one framed row per check, the footer in the bottom border.

**Rows** are wrapped like step lines with the `█` cell (every report row is finished), icons from the `[report]` block: `✓` ok, `ℹ` info, `⚠` warning, `✘` error. Results are coloured by status: `ansi-green`, `term-dim`, `ansi-yellow`, `ansi-red`. Detail lines are framed as plain content rows.

**Borders:** top `╭─ <title> ─╮` with the title bold (`title_color`), bottom `╰─ Done N checks · … ─╯` in the summary colour.

The section titles in the preview are the real ones from `extensions-rust/deps/src/format.rs`; the results are illustrative.

Source: `render_report_boxed` (`crates/theme/src/configurable/report.rs`).
