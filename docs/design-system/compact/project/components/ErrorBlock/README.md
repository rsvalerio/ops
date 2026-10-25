# ErrorBlock

The detail under a failed step: exit status and the last lines of stderr, bracketed by `╭─` and `╰─`.

**Anatomy:** `cell-1` left pad · a `cell-4` gutter (compact has no `rail`, so the gutter is icon column + 3 spaces) · `╭─` top · `│ ` message lines · `│ stderr (last N lines):` · `│   ` each stderr line · `╰─` bottom.

**Colour:** `error_block.color` is empty in compact: the brackets are `term-fg`. The failure is told by `✗` on the step line above.

**Content:** stderr is sanitised before rendering (escape sequences neutralised). Up to five stderr lines (`DEFAULT_STDERR_TAIL_LINES`). Hidden when `output.show_error_detail = false`.

**Consumer supplies:** the error message and the captured stderr tail.

Source: `render_error_block` (`crates/theme/src/render.rs`), `[themes.compact.error_block]`.
