# Spinner

Eight braille dots cycling every 80ms in `ansi-magenta`, beside the `▓` running cell.

**Frames:** `⠁⠂⠄⡀⢀⠠⠐⠈` plus one blank rest frame (`tick_chars`), the same set as compact, in flux's colour.

**Paired with:** the cyan running message and the elapsed time right-aligned in six columns, `ansi-yellow-dim`.

**Motion:** the only animation in the theme. In docs, respect `prefers-reduced-motion` and show a still frame.

Source: `running_template` and `tick_chars` in `[themes.studio]`.
