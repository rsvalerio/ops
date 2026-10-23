# Summary

The run result set into the bottom border: `╰─ Done 2/2 in 0.54s ───╯`, or `╰─ 2 succeeded, 0 skipped, 1 failed of 3 in 0.53s ───╯`.

**Anatomy:** `╰─` · space · body · space · `─` fill · `╯`. Studio's `summary_prefix` is empty.

**Colour:** the title is `ansi-green`, `cell-bold` (`summary_color`), whatever the outcome: a failed run's border title is green too. The words carry the result.

Source: `box_bottom_border` (`crates/theme/src/configurable/boxed.rs`).
