# Spinner

The animated cell at the start of a running row: eight braille dots cycling every 80ms in `ansi-cyan`.

**Frames:** `⠁⠂⠄⡀⢀⠠⠐⠈` plus one blank rest frame (`tick_chars`). The glyph is one column wide, so the spinner cell is one cell.

**Paired with:** the running elapsed time `{elapsed:>6.dim}` (`term-dim`), e.g. `3.12s` (`format_duration`), right-aligned in six columns.

**Motion:** the only animation in the theme. In docs, respect `prefers-reduced-motion` and show a still frame.

Source: `running_template` and `tick_chars` in `[themes.compact]`; steady tick set in `crates/runner/src/display.rs`.
