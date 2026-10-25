# Spinner

A full braille block with one dot missing, the gap circling every 80ms in `ansi-magenta`.

**Frames:** `⣾⣽⣻⢿⡿⣟⣯⣷` plus one blank rest frame (`tick_chars`). One cell wide.

**Paired with:** the cyan running message and the elapsed time in `ansi-yellow-dim`, e.g. `3.12s` (`format_duration`).

**Motion:** the only animation in the theme. In docs, respect `prefers-reduced-motion` and show a still frame.

Source: `running_template` and `tick_chars` in `[themes.flux]`.
