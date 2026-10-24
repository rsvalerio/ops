# ProgressColumn

A one-cell column inside the left bar that fills as the plan runs: `░` pending, `▓` running, `█` done.

**Anatomy:** `│ ` · cell · two spaces · the step line. Done means any terminal state: succeeded, failed and skipped are all `█`, so the column reads as a solid bar of completed work; the step icon tells them apart.

**Colour:** uncoloured (`term-fg`). Report rows always use `█`.

Source: progress cell selection in `crates/runner/src/display.rs`; `wrap_step_line` (`crates/theme/src/configurable/boxed.rs`).
