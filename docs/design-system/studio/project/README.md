# ops studio

Boxed frame with live summary header and vertical progress column. Studio is one of the four built-in themes of the `ops` CLI (`classic`, `compact`, `flux`, `studio`), selected with `output.theme = "studio"` in `.ops.toml`. It is the only boxed layout (`layout_kind = "boxed"`): the whole run lives in one frame, with the plan in its top border, a progress column down its left side and the result in its bottom border.

This system documents what studio actually prints. Every specimen is the renderer's own arithmetic (`render_slot` and `wrap_step_line` at 72 columns), checked against a live `ops` run.

## Principles

- **One frame, one run.** `╭─ … ─╮` to `╰─ … ─╯`. Nothing a run prints escapes the frame: step rows, error detail and report rows all close on the right bar.
- **The borders are live.** The top border names the plan; the bottom border carries the summary and updates as the run advances.
- **Progress is a column.** `░` pending, `▓` running, `█` done fill the left edge top to bottom, so how far the run has got is readable at a glance, without counting lines.
- **The right bar never moves.** Every row is padded to the frame; running rows hold their elapsed time in a six-column field (`cell-6`). A label is truncated with `…` before the frame breaks.
- **Flux's palette, compact's calm.** Cyan labels, a faint `.` leader, dim-yellow durations, a magenta spinner; compact's dot leader and braille spinner. The error bracket is the only coloured error in any theme (`ansi-red-dim`).

## Content fundamentals

ops speaks in short terminal phrases, sentence case, no punctuation at the end.

- Top border: `⚡ Running: build, lint, test`.
- Bottom border, success: `Done 2/2 in 0.54s`.
- Bottom border, failure: `2 succeeded, 0 skipped, 1 failed of 3 in 0.53s`. Still green: the words carry the result.
- Report: title in the top border (`Dependency Check`), `Done 5 checks · 1 error, 1 warning` in the bottom.
- Durations: `0.74s`, `2m14s`, `1h2m3s`.
- Labels are the command as run (`cargo build --all-targets`) or a section title in Title Case (`Duplicate Crates`).

## Visual foundations

**Colour.** Ops never paints a background, so `term-bg` is whatever the user's terminal is. The hex values are a reference rendering of the ANSI names studio emits, in a dark and a light terminal.

| Part | Token | ANSI spec in the theme |
|---|---|---|
| Top-border title | `ansi-bright-white`, bold | `header_color = "bold bright_white"` |
| Step label, running message | `ansi-cyan` | `label_color = "cyan"`, `{msg:.cyan}` |
| `.` leader | `ansi-bright-black` | `separator_color = "bright_black"` |
| Duration, running elapsed | `ansi-yellow-dim` | `duration_color = "yellow dim"` |
| Bottom-border summary | `ansi-green`, bold | `summary_color = "bold green"` |
| Spinner | `ansi-magenta` | `{spinner:.magenta}` |
| Error bracket ├─ │ └─ | `ansi-red-dim` | `error_block.color = "red dim"` |
| Report results | `ansi-green` / `term-dim` / `ansi-yellow` / `ansi-red` | `[report]` defaults |
| Frame, progress column, icons | `term-fg` | (uncoloured) |

**Contrast, honestly.** Like flux, studio is built for dark terminals, and this system keeps the two failing pairs as the source emits them:
- `ansi-bright-white` title: 1.0:1 on a light background. Override `header_color` for light-terminal users.
- `ansi-bright-black` leader: 2.9:1 dark, 2.4:1 light. Fine for a leader that carries no information; never for text.

Every other text token holds 4.5:1 on `term-bg` in both themes.

**Type.** One monospace face on one grid. `cell` (14/17) is the reference; `cell-bold` marks the two border titles and the report title. The stack prefers JetBrains Mono (hosted on Google Fonts) and falls back to the system mono. Box-drawing and block glyphs (`─ │ ╭ █ ░ ▓`) must fill their cells so the frame joins.

**Layout.** `left_pad = 0`: the frame spans exactly `output.columns`. Each row inside spends 7 columns on chrome (`cell-7`: `│ `, the progress cell, two spaces, and ` │`), so step lines are laid out at `columns − 7`. One terminal row is `row`; the frame adds one above and one below.

## Iconography

| Status | Glyph | Progress cell | Key |
|---|---|---|---|
| Pending | `◇` | `░` | `icon_pending` |
| Running | spinner `⠁⠂⠄⡀⢀⠠⠐⠈` | `▓` | `icon_running = ""`, `tick_chars` |
| Succeeded | `✓` | `█` | `icon_succeeded` |
| Failed | `✖` | `█` | `icon_failed` |
| Skipped | `⊘` | `█` | `icon_skipped` |

Frame glyphs: `╭─ ╮ │ ╰─ ╯` outside, `├─ └─` for the error bracket. `⚡` prefixes the plan title. Report rows use the shared `[report]` glyphs: `✓` ok, `ℹ` info, `⚠` warning, `✘` error.

There is no logo. Use the name `ops studio` set in `display` type.

## The theme, as configured

```toml
[themes.studio]
description = "Boxed frame with live summary header and vertical progress column"
icon_pending = "◇"
icon_running = ""
icon_succeeded = "✓"
icon_failed = "✖"
icon_skipped = "⊘"
separator_char = "."
step_indent = "  "
running_template = "│ ▓  {spinner:.magenta}{msg:.cyan} {elapsed:>6.yellow.dim} │"
tick_chars = "⠁⠂⠄⡀⢀⠠⠐⠈ "
running_template_overhead = 15
plan_header_style = "plain"
plan_header_prefix = "⚡ "
header_color = "bold bright_white"
label_color = "cyan"
separator_color = "bright_black"
duration_color = "yellow dim"
summary_prefix = ""
summary_separator = ""
summary_color = "bold green"
left_pad = 0
layout_kind = "boxed"

[themes.studio.error_block]
top = "├─"
mid = "│"
bottom = "└─"
rail = "│"
color = "red dim"
```

Source: `crates/core/src/.default.ops.toml`. Override any key by redefining `[themes.studio]` in a project's `.ops.toml`. If you change `running_template`, recount `running_template_overhead`: ops warns when it is below the template's literal width.

## Using the components

The components are terminal specimens styled by `components/bundle.css`: wrap lines in `.ops-term`, one line per `.ops-line`, and colour runs with one `sgr-<code>` class per SGR code of the theme's colour spec (`"bold green"` → `sgr-1 sgr-32`; `sgr-33 sgr-2` is `ansi-yellow-dim`). Pin every non-ASCII glyph in `.ops-c` (two-cell emoji in `.ops-c2`) so fallback fonts keep the grid. Keep text whitespace exact; the layout is in the spaces.

The previews are generated from the theme config by `cargo run -p ops-theme --example design_specimens` (see `docs/design-system/README.md` in the ops repository); edit this README, the tokens, the component READMEs and the cover by hand.

## Not synced

- No font files: ops uses the user's terminal font. JetBrains Mono is named as a hosted face only.
- No logo or icon files exist in the repository.
- Components are static renditions (no bundle). The CLI is Rust, so there is no component library to build.
- The other themes (`compact`, `classic` and `flux`) have their own systems.
