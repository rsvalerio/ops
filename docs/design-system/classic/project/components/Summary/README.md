# Summary

The last leaf of the tree: `└── Done 2/2 in 0.54s` when everything passed, `└── 2 succeeded, 0 skipped, 1 failed of 3 in 0.55s` when anything failed.

**Anatomy:** `cell-1` left pad · `summary_prefix = "└── "` · body. While a run is live on a TTY, `summary_separator = "│"` holds the trunk open above the footer.

**Durations** follow `format_duration`: `0.74s`, `2m14s`, `1h2m3s`.

**Colour:** `summary_color` is empty in classic: `term-fg`.

Source: `ConfigurableTheme::render_summary` / `render_summary_separator` (`crates/theme/src/configurable.rs`).
