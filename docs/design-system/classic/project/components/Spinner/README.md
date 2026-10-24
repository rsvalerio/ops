# Spinner

The animated cell on a running branch: an ASCII bar turning `| / - \` every 80ms in `ansi-cyan`.

**Frames:** `|/-\` plus one blank rest frame (`tick_chars`). Pure ASCII, so it renders in any font and any log viewer.

**Paired with:** the running elapsed time `{elapsed:>6.dim}` (`term-dim`), e.g. `3.12s` (`format_duration`), right-aligned in six columns.

**Motion:** the only animation in the theme. In docs, respect `prefers-reduced-motion` and show a still frame.

Source: `running_template` and `tick_chars` in `[themes.classic]`; steady tick set in `crates/runner/src/display.rs`.
