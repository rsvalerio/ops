# ErrorBlock

The detail under a failed step, hung off the trunk: exit status and the last lines of stderr inside a `┌─` / `└─` bracket.

**Anatomy:** `cell-1` left pad · `│` rail (the trunk continues) · three spaces · `┌─` top · `│ ` message lines · `│ stderr (last N lines):` · `│   ` each stderr line · `└─` bottom.

**Colour:** `error_block.color` is empty in classic: the brackets are `term-fg`. The failure is told by `✖` on the step line above.

**Content:** stderr is sanitised before rendering (escape sequences neutralised). Up to five stderr lines (`DEFAULT_STDERR_TAIL_LINES`). Hidden when `output.show_error_detail = false`.

**Consumer supplies:** the error message and the captured stderr tail.

Source: `render_error_block` (`crates/theme/src/render.rs`), `[themes.classic.error_block]`.
