# Visual Component Catalog

Reference for every visual element rendered by `ops` CLI output. Components are listed in render order.

---

## 1. Plan Header

**What:** Opening line announcing which commands will run, surrounded by blank lines.

**Visual example:**
```text

Running: build, clippy, test

```

**Theme variations:** None (identical in classic and compact).

**Configuration:** None. Content is derived from the command IDs in the run plan.

**Implementation:** `StepLineTheme::render_plan_header()` in `crates/theme/src/lib.rs:154-157` (trait default). `ConfigurableTheme` override at `crates/theme/src/lib.rs:63-77`. Default trait method joins command IDs with `, ` and wraps in blank lines.

---

## 2. Pending Step Line

**What:** Initial state for every step before execution begins: icon + label + dot padding.

**Visual example (classic):**
```text
  ○  cargo build --all-targets .......................................
  ○  cargo clippy --all-targets -- -D warnings ........................
  ○  cargo test --all-targets .........................................
```

**Theme variations:** Icon is `○` in both classic and compact.

**Configuration:**
- `output.columns` — total line width used to calculate dot-fill length.

**Implementation:**
- Full line: `StepLineTheme::render()` in `crates/theme/src/lib.rs:225-239`
- Prefix (indent + icon + label): `StepLineTheme::render_prefix()` in `crates/theme/src/lib.rs:242-253`
- Separator dots: `StepLineTheme::render_separator()` in `crates/theme/src/lib.rs:256-282`

---

## 3. Spinner

**What:** Animated character shown while a step is running.

**Visual example (compact):**
```text
  ⠁ cargo build --all-targets ........................... 3s
  ⠂ cargo clippy --all-targets -- -D warnings ........... 1s
```

**Visual example (classic):**
```text
├── | cargo build --all-targets ........................ 3s
├── / cargo clippy --all-targets -- -D warnings ........ 1s
```

**Theme variations:**
- Classic: `|/-\` (4 ASCII frames + 1 blank rest frame), template `"├── {spinner:.cyan}{msg} {elapsed:.dim}"`
- Compact: `⠁⠂⠄⡀⢀⠠⠐⠈` (8 braille frames + 1 blank rest frame), template `"  {spinner:.cyan}{msg} {elapsed:.dim}"`

Both cycle at 80ms via `enable_steady_tick`.

**Configuration:** Spinner characters and template are defined per theme in `ThemeConfig` (`crates/core/src/config/theme_types.rs:88-132`).

**Implementation:**
- Style: `running_style` in `ProgressDisplay::new_with_tty_check()` at `crates/runner/src/display.rs:144-152`
- Templates and tick chars: `ThemeConfig::classic()` at `crates/core/src/config/theme_types.rs:97-98`, `ThemeConfig::compact()` at `crates/core/src/config/theme_types.rs:123-124`
- Template overhead: `running_template_overhead()` method in `crates/theme/src/lib.rs:205-207`, configured per theme via `ThemeConfig.running_template_overhead` field at `crates/core/src/config/theme_types.rs:70`

---

## 4. Step Status Icons

**What:** Per-status icon rendered at the left of each step line.

| Status      | Classic | Compact | Display width |
|-------------|---------|---------|---------------|
| Pending     | `○`     | `○`     | 1             |
| Running     | *(none — spinner takes this slot)* | *(same)* | 0 |
| Succeeded   | `✅`    | `✓`     | 2 / 1         |
| Failed      | `✗`     | `✗`     | 1             |
| Skipped     | `—`     | `—`     | 1             |

Icons are right-padded to `icon_column_width()` so labels stay vertically aligned across all statuses.

**Configuration:** `output.theme` selects the icon set.

**Implementation:**
- Classic icons: `ThemeConfig::classic()` at `crates/core/src/config/theme_types.rs:88-111`
- Compact icons: `ThemeConfig::compact()` at `crates/core/src/config/theme_types.rs:114-132`
- Icon resolution: `ConfigurableTheme::status_icon()` at `crates/theme/src/lib.rs:35-41`
- Alignment padding: `StepLineTheme::icon_column_width()` at `crates/theme/src/lib.rs:174-181`

---

## 5. Separator Dots

**What:** Dot-fill between the label and elapsed time, padding the line to the configured column width.

**Visual example:**
```text
  ✅ cargo build --all-targets ........................... 0.35s
                                ^^^^^^^^^^^^^^^^^^^^^^^^^
                                    separator dots
```

**Rules:**
- Default character: `.` (overridable via `separator_char()`)
- Minimum 3 characters, fills remaining width from the `columns` budget
- Running steps: last dot replaced with a space so `{elapsed}` from the indicatif template has a clean gap

**Theme variations:** None (both themes use `.`).

**Configuration:** `output.columns` controls available width.

**Implementation:** `StepLineTheme::render_separator()` in `crates/theme/src/lib.rs:256-282`. Uses `running_template_overhead()` to subtract the indicatif template's fixed-width elements for running steps.

---

## 6. Elapsed Timer

**What:** Duration display at the right edge of a step line.

**Visual examples:**
```text
0.35s        (finished step, rendered by theme)
3s           (running step, rendered by indicatif {elapsed:.dim})
```

**Format:** `{:.2}s` — two decimal places when rendered by the theme (finished steps). While running, indicatif's built-in `{elapsed}` token shows whole seconds in dim style.

**Theme variations:** None.

**Configuration:** None.

**Implementation:**
- Finished: `StepLineTheme::format_elapsed()` in `crates/theme/src/lib.rs:165-167`, with standalone `format_duration()` helper at `crates/theme/src/lib.rs:19-33`
- Running: `{elapsed:.dim}` in the spinner template (see `ThemeConfig` in `crates/core/src/config/theme_types.rs`)

---

## 7. Error Detail Box

**What:** Box-drawing block shown below a failed step with exit info and stderr tail.

**Visual example (classic theme):**
```text
  ✗  cargo test --all-targets ........................... 0.42s
     ╭─
     │ exit status: 101
     │ stderr (last 5 lines):
     │   thread 'main' panicked at 'assertion failed'
     │   note: run with `RUST_BACKTRACE=1` for a backtrace
     │   error: test failed, to rerun pass `--lib`
     │   error: could not compile `my-crate`
     │   error: process didn't exit successfully
     ╰─
```

**Structure:**
- `╭─` top border
- `│ <message>` exit message line
- `│ stderr (last N lines):` header (only when stderr lines exist)
- `│   <line>` indented stderr lines
- `╰─` bottom border

**Gutter:** Aligned with label column — `icon_column_width() + 3` spaces (2-char line indent + icon width + 1 space).

**Stderr capture:** Last 5 lines of stderr, defined as `STDERR_TAIL_LINES` constant in `crates/runner/src/display.rs:22`.

**Theme variations:** Gutter width differs because `icon_column_width()` varies (classic: 5 spaces `"     "`, compact: 4 spaces `"    "`).

**Configuration:** `output.show_error_detail` (boolean, default `true`). When `false`, the error box is suppressed entirely.

**Implementation:** `StepLineTheme::render_error_detail()` in `crates/theme/src/lib.rs:216-222`, with `render_error_block()` helper at `crates/theme/src/lib.rs:286-317`. Error detail toggle check at `crates/runner/src/display.rs:371`.

---

## 8. Summary Separator

**What:** Visual break between the last step line and the footer/summary. Rendered upfront when the plan starts and remains visible throughout execution.

**Visual example:**
```text
  ✅ cargo test --all-targets ........................... 0.80s
│                                                             <-- separator
└── Done 5/5 in 23.62s
```

**Theme variations:** Classic uses `│` (tree connector). Compact returns empty string (rendered as blank line).

**Configuration:** None. Overridable per theme via `render_summary_separator()`.

**Implementation:** `StepLineTheme::render_summary_separator()` in `crates/theme/src/lib.rs`. Created as a progress bar in `ProgressDisplay::on_plan_started()`. TTY-only during progress; written to stderr on run finish for non-TTY.

---

## 9. Footer / Summary Line

**What:** Progress footer shown from the start of execution, updated as steps complete, and finalized with elapsed time when the run ends.

**Visual examples (during execution):**
```text
└── Done 0/5…      (plan just started)
└── Done 3/5…      (3 steps completed)
```

**Visual examples (final):**
```text
└── Done 5/5 in 23.62s       (all steps succeeded)
└── Failed 3/5 in 15.97s     (one or more steps failed)
```

**Format:** `"{summary_prefix}{Done|Failed} {completed}/{total} in {elapsed}"`.

**Theme variations:** `summary_prefix()` controls the line prefix (`"└── "` for classic, empty for compact).

**Configuration:** None.

**Implementation:** Footer bar created in `ProgressDisplay::on_plan_started()`, updated in `finish_step()`, finalized in `on_run_finished()`. `StepLineTheme::summary_prefix()` in `crates/theme/src/lib.rs`.

---

## Full Output Example

A complete classic-theme run with all components annotated:

```text
                                              # Component
Running: build, clippy, test                  # [1] Plan Header

  ⠁ cargo build --all-targets ...... 2s       # [3] Spinner + [5] Dots + [6] Timer (running)
  ○  cargo clippy -- -D warnings ......       # [2] Pending Step Line + [4] Icon (○)
  ○  cargo test --all-targets .........       # [2] Pending Step Line
                                              # [8] Summary Separator
Done 0/3…                                     # [9] Footer (progress)

  ✅ cargo build --all-targets ...... 12.35s  # [4] Icon (✅) + [5] Dots + [6] Timer
  ✅ cargo clippy -- -D warnings .... 3.20s
  ✗  cargo test --all-targets ....... 0.42s   # [4] Icon (✗)
     ╭─                                       # [7] Error Detail Box
     │ exit status: 101                       #
     │ stderr (last 2 lines):                 #
     │   error: test failed                   #
     │   error: could not compile             #
     ╰─                                       #
                                              # [8] Summary Separator (same bar)
Failed 2/3 in 15.97s                   # [9] Footer (final summary)
```

---

## TTY vs Non-TTY Rendering

| Aspect | TTY (terminal) | Non-TTY (CI / piped) |
|--------|---------------|----------------------|
| Progress bars | `MultiProgress` with `ProgressDrawTarget::stderr()` | `ProgressDrawTarget::hidden()` |
| Spinner animation | Visible, 80ms tick | Not rendered |
| Step updates | In-place redraws via indicatif | `writeln!` to stderr on each step finish/fail |
| Error detail | Inserted as progress bars after the failed step | `writeln!` to stderr |
| Footer progress | Visible, updates in-place as steps complete | Not rendered (TTY-only) |
| Summary | Footer bar finalized via `finish_with_message` | `writeln!` to stderr |

**Implementation:** TTY detection via `is_stderr_tty()` at `crates/runner/src/display.rs:114-116`. Non-TTY fallback in `ProgressDisplay::emit_line()` at `crates/runner/src/display.rs:189-199`.

---

## Configuration Reference

All `[output]` knobs and the components they affect:

| Config key | Type | Default | Components affected |
|-----------|------|---------|---------------------|
| `output.theme` | `"classic"` \| `"compact"` | `"classic"` | [4] Step Status Icons |
| `output.columns` | `u16` | `80` | [2] Pending Step Line, [5] Separator Dots |
| `output.show_error_detail` | `bool` | `true` | [7] Error Detail Box |

**Config sources** (later overrides earlier): embedded default → global `~/.config/ops/config.toml` → local `.ops.toml` → environment `CARGO_OPS_*`.

---

## Theme Comparison

Side-by-side icon rendering for classic vs compact:

```text
Classic                                    Compact
───────                                    ───────
  ○  cargo build (pending)                   ○ cargo build (pending)
  ✅ cargo build ........... 0.35s           ✓ cargo build ............ 0.35s
  ✗  cargo test ............ 0.42s           ✗ cargo test ............. 0.42s
  —  cargo fmt (skipped)                     — cargo fmt (skipped)
```

Key difference: Classic uses `✅` (width 2) for success; compact uses `✓` (width 1). This changes icon-column padding and error-detail gutter width.
