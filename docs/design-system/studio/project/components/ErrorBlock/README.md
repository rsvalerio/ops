# ErrorBlock

The failure detail branching off the frame's left bar: exit status and stderr inside a `├─` / `└─` bracket drawn in dim red.

**Anatomy:** the frame bar `│` doubles as the `rail` · an injected indent so the bracket lines up under the step icon · `├─` top · `│ ` message lines · `│ stderr (last N lines):` · `│   ` stderr lines · `└─` bottom · right pad · ` │`.

**Colour:** `error_block.color = "red dim"` → `ansi-red-dim` on the bracket glyphs only; the message and stderr stay `term-fg`. Studio is the only built-in theme that colours the error block.

**Content:** stderr is sanitised before rendering. Up to five stderr lines. Hidden when `output.show_error_detail = false`.

**Consumer supplies:** the error message and the captured stderr tail.

Source: `render_error_detail` (`crates/theme/src/configurable/boxed.rs`), `[themes.studio.error_block]`.
