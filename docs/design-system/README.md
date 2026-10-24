# Theme design systems

One design system per built-in output theme, published as private Design System
pages on claude.ai:

| Theme | Page |
|---|---|
| `compact` | [ops — Compact Theme](https://claude.ai/artifact/6wGKpnzpK8fsWGYrS9VxKP) |
| `classic` | [ops — Classic Theme](https://claude.ai/artifact/WUoCUjuoWhcmYTMxReBVvY) |
| `flux` | [ops — Flux Theme](https://claude.ai/artifact/Jeb8MChmMhVjCXMwDTJGLe) |
| `studio` | [ops — Studio Theme](https://claude.ai/artifact/4syGPFfx9dgmz5C9dNsvrn) |

Each system documents what its theme actually prints: colour tokens for a
dark and a light terminal, the monospace type, spacing counted in terminal
columns, a brand-book README, and one card per output component (plan
header, step line, spinner, error block, summary, `ops deps` report, a whole
run; `studio` adds the box frame and progress column). The pages are private
until shared from their Share menu.

## Layout

Each `<theme>/project/` directory holds one system's files, at the paths the
published page expects:

| Path | Kind |
|---|---|
| `README.md` | hand-written: the theme's brand book |
| `tokens.json` | hand-written: colours, type, spacing, radii |
| `components/<Component>/README.md` | hand-written: component guidelines |
| `components/Cover/preview.html` | hand-written: the cover |
| `components/<Component>/preview.html` | **generated** specimen |
| `components/bundle.css` | **generated** copy of [`bundle.css`](bundle.css) |

[`bundle.css`](bundle.css) here is the one stylesheet all four systems share.
Edit it, not the copies.

## Regenerating the specimens

The specimens are rendered from the theme config embedded in
`crates/core/src/.default.ops.toml` through the real `ConfigurableTheme`, so
they follow any theme change. Rerun the generator after changing a theme, its
renderer, or `bundle.css`:

```bash
cargo run -p ops-theme --example design_specimens
```

The generator is [`crates/theme/examples/design_specimens.rs`](../../crates/theme/examples/design_specimens.rs).
Every line comes from the renderer (`render`, `render_plan_header`,
`render_error_detail`, `render_report`, the box borders, `wrap_step_line`), and
running rows are drawn by indicatif from the theme's `running_template`, as the
runner draws them. Because the renderer emits colour only when stderr is a
terminal, specimens are rendered plain and each part gets one `sgr-<code>`
class per SGR code its colour spec compiles to (`precompute_sgr_prefix`).

A run fails instead of writing a theme's previews when:

- a specimen line is wider than the 72-column budget,
- a running row's layout disagrees with what its `running_template` places, or
- the theme uses a colour its `tokens.json` does not define, or that
  `bundle.css` has no rule for (the page would silently fall back to the
  default foreground).

The example's tests run with the workspace tests (`[[example]] test = true`
in `crates/theme/Cargo.toml`). One of them, `committed_previews_are_up_to_date`,
renders everything and compares it with the files here, so a theme or
renderer change that skips regeneration fails `ops next` / CI.

Then update the hand-written files if the change affects them: the
`[themes.<name>]` block quoted in each `README.md`, and the component READMEs
that describe templates, glyphs or colours.

## Publishing a change

The published pages are claude.ai artifacts; a script cannot update them. In
Claude Code, ask for the change to be published, naming the theme and its
page, for example:

> Republish docs/design-system/flux to https://claude.ai/artifact/Jeb8MChmMhVjCXMwDTJGLe

Only the files under `<theme>/project/` are published. The page's own index
(`project/design-system.json`) lives on the page and is not kept here; it is
read back from the page and updated when publishing.

## Known gaps

- `flux` and `studio` print their plan header in bold bright white, which is
  almost invisible on light terminals (contrast 1.0:1), and their separator
  rule is bright black (below 3:1). The tokens keep these values as the
  themes emit them and say so in their usage notes.
- The summary colour applies whatever the outcome, so a failed run's summary
  is green in `flux` and `studio`.
- A `studio` running label starts two columns left of the finished labels: its
  `running_template` has no `step_indent`.
