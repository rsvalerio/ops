# ops flux

Modern automation: emoji status, heavy flow line, full color palette. Flux is one of the four built-in themes of the `ops` CLI (`classic`, `compact`, `flux`, `studio`), selected with `output.theme = "flux"` in `.ops.toml`. It is the loud one: a 🚀 launch, emoji verdicts, a cyan flow of labels and a ⚡ finish.

This system documents what flux actually prints. Every specimen is the renderer's own arithmetic (`ConfigurableTheme::render_slot` at 72 columns), checked against a live `ops` run.

## Principles

- **Every part has its colour.** Header bright white, labels cyan, rules bright black, durations dim yellow, summary green, spinner magenta. Flux is the only built-in theme where the eye can find a part by hue alone.
- **Emoji say the verdict.** ✅ and 💥 are readable from across the room. They are two cells wide, so the icon column is two cells (`cell-2`) and one-cell icons (⏸ ⏭) are padded to keep labels aligned.
- **A heavy flow line.** `separator_char = "━"` runs each step to its duration in `ansi-bright-black`: heavy in shape, quiet in tone, so the label and time stay in front.
- **Bold means a boundary.** The header, the summary and report titles are bold; nothing inside the run is.
- **Round only with glyphs.** `╭─` `╰─` around an error; everything else is square (`radius-none`).

## Content fundamentals

ops speaks in short terminal phrases, sentence case, no punctuation at the end. Flux adds one emoji at each boundary and none in the text.

- Plan header: `🚀 Running: build, lint, test`.
- Success summary: `⚡ Done 2/2 in 0.54s`.
- Failure summary: `⚡ 2 succeeded, 0 skipped, 1 failed of 3 in 0.55s`. Still green: the words carry the result.
- Report footer: `⚡ Done 5 checks · 1 error, 1 warning`.
- Durations: `0.74s`, `2m14s`, `1h2m3s`.
- Labels are the command as run (`cargo build --all-targets`) or a section title in Title Case (`Duplicate Crates`).

## Visual foundations

**Colour.** Ops never paints a background, so `term-bg` is whatever the user's terminal is. The hex values are a reference rendering of the ANSI names flux emits, in a dark and a light terminal.

| Part | Token | ANSI spec in the theme |
|---|---|---|
| Plan header | `ansi-bright-white`, bold | `header_color = "bold bright_white"` |
| Step label, running message | `ansi-cyan` | `label_color = "cyan"`, `{msg:.cyan}` |
| ━ rule | `ansi-bright-black` | `separator_color = "bright_black"` |
| Duration, running elapsed | `ansi-yellow-dim` | `duration_color = "yellow dim"` |
| Summary | `ansi-green`, bold | `summary_color = "bold green"` |
| Spinner | `ansi-magenta` | `{spinner:.magenta}` |
| Report results | `ansi-green` / `term-dim` / `ansi-yellow` / `ansi-red` | `[report]` defaults |
| Icons, ▸, error block, ⚡ | `term-fg` | (uncoloured) |

**Contrast, honestly.** Flux is built for dark terminals. Two of its choices fail on light ones, and this system keeps them as the source emits them:
- `ansi-bright-white` header: 1.0:1 on a light background. The header vanishes. Override `header_color` for light-terminal users.
- `ansi-bright-black` rule: 2.9:1 dark, 2.4:1 light. Acceptable for a leader that carries no information; never use it for text.

Every other text token holds 4.5:1 on `term-bg` in both themes.

**Type.** One monospace face on one grid. `cell` (14/17) is the reference; `cell-bold` marks header, summary and report title. The stack prefers JetBrains Mono (hosted on Google Fonts) and falls back to the system mono. Emoji come from the system emoji font and must be held to two cells (the specimens pin them with `.ops-c2`).

**Layout.** One terminal row is `row` (17px at the reference cell). Lines are composed: left pad (`cell-1`), `  ▸ ` (`cell-4`), icon column (`cell-2`), one space, label, rule, one space, trailing slot. The error gutter is `cell-5`.

## Iconography

| Status | Glyph | Width | Key |
|---|---|---|---|
| Pending | `⏸` | 1 (+1 pad) | `icon_pending` |
| Running | spinner `⣾⣽⣻⢿⡿⣟⣯⣷` | 1 | `icon_running = ""`, `tick_chars` |
| Succeeded | `✅` | 2 | `icon_succeeded` |
| Failed | `💥` | 2 | `icon_failed` |
| Skipped | `⏭` | 1 (+1 pad) | `icon_skipped` |

Boundary marks: `🚀` (header prefix), `⚡` (summary prefix), `▸` (step indent). Report rows use the shared `[report]` glyphs: `✓` ok, `ℹ` info, `⚠` warning, `✘` error.

There is no logo. Use the name `ops flux` set in `display` type.

## The theme, as configured

```toml
[themes.flux]
description = "Modern automation: emoji status, heavy flow line, full color palette"
icon_pending = "⏸"
icon_running = ""
icon_succeeded = "✅"
icon_failed = "💥"
icon_skipped = "⏭"
separator_char = "━"
step_indent = "  ▸ "
running_template = "  ▸ {spinner:.magenta}{msg:.cyan} {elapsed:>6.yellow.dim}"
tick_chars = "⣾⣽⣻⢿⡿⣟⣯⣷ "
running_template_overhead = 12
plan_header_style = "plain"
plan_header_prefix = "🚀 "
header_color = "bold bright_white"
label_color = "cyan"
separator_color = "bright_black"
duration_color = "yellow dim"
summary_prefix = "⚡ "
summary_separator = ""
summary_color = "bold green"
left_pad = 1

[themes.flux.error_block]
top = "╭─"
mid = "│"
bottom = "╰─"
rail = ""
```

Source: `crates/core/src/.default.ops.toml`. Override any key by redefining `[themes.flux]` in a project's `.ops.toml`.

## Using the components

The components are terminal specimens styled by `components/bundle.css`: wrap lines in `.ops-term`, one line per `.ops-line`, and colour runs with one `sgr-<code>` class per SGR code of the theme's colour spec (`"bold green"` → `sgr-1 sgr-32`; `sgr-33 sgr-2` is `ansi-yellow-dim`). Pin every non-ASCII glyph in `.ops-c` (two-cell emoji in `.ops-c2`) so fallback fonts keep the grid. Keep text whitespace exact; the layout is in the spaces.

The previews are generated from the theme config by `cargo run -p ops-theme --example design_specimens` (see `docs/design-system/README.md` in the ops repository); edit this README, the tokens, the component READMEs and the cover by hand.

## Not synced

- No font files: ops uses the user's terminal font and emoji font. JetBrains Mono is named as a hosted face only.
- No logo or icon files exist in the repository; the emoji are the system's.
- Components are static renditions (no bundle). The CLI is Rust, so there is no component library to build.
- The other themes (`compact` and `classic` have their own systems; `studio`) are not in this system.
