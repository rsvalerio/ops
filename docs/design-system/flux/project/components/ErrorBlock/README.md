# ErrorBlock

The detail under a failed step: exit status and the last lines of stderr, bracketed by `╭─` and `╰─`.

**Anatomy:** `cell-1` left pad · a `cell-5` gutter (flux has no `rail`, so the gutter is the two-cell icon column + 3 spaces) · `╭─` top · `│ ` message lines · `│ stderr (last N lines):` · `│   ` each stderr line · `╰─` bottom.

**Colour:** `error_block.color` is empty: `term-fg`. Flux shouts the failure with 💥 on the step line instead.

**Content:** stderr is sanitised before rendering. Up to five stderr lines. Hidden when `output.show_error_detail = false`.

**Consumer supplies:** the error message and the captured stderr tail.

Source: `render_error_block` (`crates/theme/src/render.rs`), `[themes.flux.error_block]`.
