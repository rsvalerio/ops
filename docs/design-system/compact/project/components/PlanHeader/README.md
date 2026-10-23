# PlanHeader

The line announcing which commands a run will execute: `Running: build, lint, test`, framed by one blank row above and below.

**When it shows:** once, at the start of every `ops` run with a plan.

**Anatomy:** left pad (`cell-1`) · `plan_header_prefix` (empty in compact) · `Running: ` · command ids joined by `, `.

**Colour:** `header_color` is empty in compact, so the line is `term-fg`.

**Consumer supplies:** the command ids of the plan, in plan order.

Source: `ConfigurableTheme::render_plan_header` (`crates/theme/src/configurable.rs`), `PlanHeaderStyle::Plain`.
