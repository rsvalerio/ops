# PlanHeader

The root of the tree: `┌ Running: build, lint, test`, then a `│` row that leads into the first step.

**When it shows:** once, at the start of every `ops` run with a plan, after one blank row.

**Anatomy:** left pad (`cell-1`) · `┌ Running: ` · command ids joined by `, ` · next row: `│` in the trunk column.

**Wrapping:** a header wider than `output.columns` wraps between ids, never inside one; each line but the last ends in `,`. Continuation rows keep the `│` trunk and indent the ids under the first one (`│` plus 10 spaces), so the tree stays joined however long the plan. An id too wide for a row on its own is truncated with `…`.

**Colour:** `header_color` is empty in classic, so the line is `term-fg`.

**Consumer supplies:** the command ids of the plan, in plan order.

Source: `ConfigurableTheme::render_plan_header` (`crates/theme/src/configurable.rs`), `PlanHeaderStyle::Tree`.
