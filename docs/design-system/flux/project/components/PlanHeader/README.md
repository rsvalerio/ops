# PlanHeader

The launch line: `🚀 Running: build, lint, test` in bold bright white, framed by one blank row above and below.

**Anatomy:** left pad (`cell-1`) · `plan_header_prefix = "🚀 "` · `Running: ` · command ids joined by `, `. The prefix is inside the coloured run.

**Colour:** `header_color = "bold bright_white"` → `ansi-bright-white`, `cell-bold`. On a light terminal this is near-invisible (see the token's note); a project that targets light terminals should override `header_color`.

**Consumer supplies:** the command ids of the plan, in plan order.

Source: `ConfigurableTheme::render_plan_header` (`crates/theme/src/configurable.rs`), `PlanHeaderStyle::Plain`.
