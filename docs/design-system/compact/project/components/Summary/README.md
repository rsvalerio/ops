# Summary

The final line of a run: `Done 2/2 in 0.54s` when everything passed, `2 succeeded, 0 skipped, 1 failed of 3 in 0.55s` when anything failed.

**Anatomy:** `cell-1` left pad · `summary_prefix` (empty in compact) · body. Compact has no `summary_separator`, so nothing visible sits between the last step and the summary.

**Durations** follow `format_duration`: `0.74s`, `2m14s`, `1h2m3s`.

**Colour:** `summary_color` is empty in compact: `term-fg`.

Source: `ConfigurableTheme::render_summary` / `render_summary_text` (`crates/theme/src/configurable.rs`).
