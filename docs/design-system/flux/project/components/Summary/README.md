# Summary

The closing line: `⚡ Done 2/2 in 0.54s`, or `⚡ 2 succeeded, 0 skipped, 1 failed of 3 in 0.55s`, in bold green.

**Anatomy:** `cell-1` left pad · `summary_prefix = "⚡ "` (uncoloured) · body in `ansi-green`, `cell-bold`.

**Known quirk:** `summary_color` is applied whatever the outcome, so a failed run's summary is green too. The words carry the result (`1 failed`); colour does not.

**Durations** follow `format_duration`: `0.74s`, `2m14s`, `1h2m3s`.

Source: `ConfigurableTheme::render_summary_text` (`crates/theme/src/configurable.rs`).
