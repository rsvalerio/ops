# Report

Command output shaped like a run, used by `ops deps`: a bold title, one row per check with a coloured result, and a ⚡ footer.

**Rows** reuse the StepLine layout (`  ▸ `, cyan label, ━ rule) with the shared `[report]` icons: `✓` ok, `ℹ` info, `⚠` warning, `✘` error. They are one cell wide and padded to flux's two-cell icon column. The result is coloured by status: `ansi-green` ok, `term-dim` info, `ansi-yellow` warning, `ansi-red` error.

**Title:** `cell-bold` (`title_color = "bold"`), `term-fg`.

**Footer:** the Summary chrome: `⚡ Done N checks · …` in bold green.

The section titles in the preview are the real ones from `extensions-rust/deps/src/format.rs`; the results are illustrative.

Source: `ConfigurableTheme::render_report` (`crates/theme/src/configurable/report.rs`).
