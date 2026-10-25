# ops compact

Minimal with dot separators. Compact is one of the four built-in themes of the `ops` CLI (`classic`, `compact`, `flux`, `studio`), selected with `output.theme = "compact"` in `.ops.toml`. It is the quiet one: no tree connectors, no emoji, no colour on anything that is not moving or reporting a verdict.

This system documents what compact actually prints. Every specimen is the renderer's own arithmetic (`ConfigurableTheme::render_slot` at 72 columns), checked against a live `ops` run.

## Principles

- **Glyphs carry state, not colour.** ✓ ✗ ○ — are read in `term-fg`. A step line stays legible with `NO_COLOR` set and in a CI log, because nothing depends on hue.
- **Colour only for motion and verdicts.** The spinner is `ansi-cyan`; the running elapsed time is `term-dim`. Report results (`ops deps`) use `ansi-green`, `term-dim`, `ansi-yellow`, `ansi-red`. Nothing else is coloured.
- **The dot leader is the identity.** `separator_char = "."` fills each line to the column budget so durations right-align. At least three dots (`cell-3`), always.
- **Measured in columns.** Spacing is counted in cells: one cell of left margin (`cell-1`), two of step indent (`cell-2`), a four-cell gutter under failures (`cell-4`). No line exceeds its budget; labels are truncated with `…` instead.
- **Round only with glyphs.** The only curves are `╭─` and `╰─` around an error block. Everything else is square (`radius-none`).

## Content fundamentals

ops speaks in short terminal phrases, sentence case, no punctuation at the end.

- Plan header: `Running: build, lint, test`: the verb, then command ids joined by `, `.
- Success summary: `Done 2/2 in 0.54s`.
- Failure summary: `2 succeeded, 0 skipped, 1 failed of 3 in 0.55s`. Counts come before the total, the total before the time.
- Report footer: `Done 5 checks · 1 error, 1 warning`. A middle dot separates the count from the issues.
- Durations: `0.74s` under a minute (two decimals), `2m14s`, `1h2m3s`. Never `--` unless the value is not finite.
- Labels are the command as run (`cargo build --all-targets`) or a section title in Title Case (`Duplicate Crates`).

## Visual foundations

**Colour.** Ops never paints a background, so `term-bg` is whatever the user's terminal is. The hex values here are a reference rendering of ANSI names, in a dark and a light terminal; real terminals substitute their own palette. Text tokens hold 4.5:1 on `term-bg` and `term-surface` in both themes. `term-border` is a decorative frame for doc specimens only.

| Role | Token | ANSI spec in the theme |
|---|---|---|
| Everything uncoloured | `term-fg` | (empty) |
| Running elapsed, Info result | `term-dim` | `dim` |
| Spinner | `ansi-cyan` | `{spinner:.cyan}` |
| Ok result | `ansi-green` | `color_ok = "green"` |
| Warning result | `ansi-yellow` | `color_warning = "yellow"` |
| Error result | `ansi-red` | `color_error = "red"` |
| Report title | `term-fg`, `cell-bold` | `title_color = "bold"` |

**Type.** One monospace face on one grid. `cell` (14/17) is the reference. `cell-bold` is the only weight change, for report titles. The stack prefers JetBrains Mono (hosted on Google Fonts) and falls back to the system mono. Braille spinner frames may come from a fallback face; that is expected.

**Layout.** One terminal row is `row` (17px at the reference cell). Blank rows frame the plan header (one above, one below). Lines are composed left to right as: left pad, indent, icon column, one space, label, dot run, one space, trailing slot.

## Iconography

Compact's icons are text glyphs from the theme config, padded to the widest one (all are one column wide, so the icon column is one cell).

| Status | Glyph | Key |
|---|---|---|
| Pending | `○` | `icon_pending` |
| Running | spinner `⠁⠂⠄⡀⢀⠠⠐⠈` | `icon_running = ""`, `tick_chars` |
| Succeeded | `✓` | `icon_succeeded` |
| Failed | `✗` | `icon_failed` |
| Skipped | `—` | `icon_skipped` |

Report rows use the shared `[report]` glyphs: `✓` ok, `ℹ` info, `⚠` warning, `✘` error.

There is no logo. Use the name `ops compact` set in `display` type.

## The theme, as configured

```toml
[themes.compact]
description = "Minimal with dot separators"
icon_pending = "○"
icon_running = ""
icon_succeeded = "✓"
icon_failed = "✗"
icon_skipped = "—"
separator_char = "."
step_indent = "  "
running_template = "  {spinner:.cyan}{msg} {elapsed:>6.dim}"
tick_chars = "⠁⠂⠄⡀⢀⠠⠐⠈ "
running_template_overhead = 10
plan_header_style = "plain"
summary_prefix = ""
summary_separator = ""
left_pad = 1

[themes.compact.error_block]
top = "╭─"
mid = "│"
bottom = "╰─"
rail = ""
```

Source: `crates/core/src/.default.ops.toml`. Override any key by redefining `[themes.compact]` in a project's `.ops.toml`.

## Using the components

The components are terminal specimens styled by `components/bundle.css`: wrap lines in `.ops-term`, one line per `.ops-line`, and colour runs with one `sgr-<code>` class per SGR code of the theme's colour spec (`"bold green"` → `sgr-1 sgr-32`; `sgr-33 sgr-2` is `ansi-yellow-dim`). Pin every non-ASCII glyph in `.ops-c` (two-cell emoji in `.ops-c2`) so fallback fonts keep the grid. Keep text whitespace exact; the layout is in the spaces.

The previews are generated from the theme config by `cargo run -p ops-theme --example design_specimens` (see `docs/design-system/README.md` in the ops repository); edit this README, the tokens, the component READMEs and the cover by hand.

## Not synced

- No font files: ops uses the user's terminal font. JetBrains Mono is named as a hosted face only.
- No logo or icon files exist in the repository.
- Components are static renditions (no bundle). The CLI is Rust, so there is no component library to build.
- The other three themes (`classic`, `flux`, `studio`) are not in this system.
