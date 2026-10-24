# PlanHeader

The plan title set into the top border: `╭─ ⚡ Running: build, lint, test ───╮`.

**Anatomy:** `╭─` · space · `plan_header_prefix = "⚡ "` · `Running: ` · ids joined by `, ` · space · `─` fill · `╮`. A title wider than the frame is truncated with `…`.

**Colour:** the title (spaces included) is `ansi-bright-white`, `cell-bold` (`header_color`). The corners and fill are `term-fg`. On light terminals the title disappears (see the token's note).

Source: `box_top_border` (`crates/theme/src/configurable/boxed.rs`).
