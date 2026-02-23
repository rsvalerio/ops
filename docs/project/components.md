# Visual Component Catalog

Reference for every visual element rendered by `cargo-ops` CLI output. Components are listed in render order.

---

## 1. Plan Header

**What:** Opening line announcing which commands will run, surrounded by blank lines.

**Visual example:**
```text

Running: build, clippy, test

```

**Theme variations:** None (identical in classic and compact).

**Configuration:** None. Content is derived from the command IDs in the run plan.

**Implementation:** `StepLineTheme::render_plan_header()` in `src/theme.rs:47-50`. Default trait method joins command IDs with `, ` and wraps in blank lines.

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
- Full line: `StepLineTheme::render()` in `src/theme.rs:117-136`
- Prefix (indent + icon + label): `StepLineTheme::render_prefix()` in `src/theme.rs:139-152`
- Separator dots: `StepLineTheme::render_separator()` in `src/theme.rs:159-186`

---

## 3. Spinner

**What:** Animated braille-dot character shown while a step is running.

**Visual example:**
```text
  ⠁ cargo build --all-targets ........................... 3s
  ⠂ cargo clippy --all-targets -- -D warnings ........... 1s
```

**Characters:** `⠁⠂⠄⡀⢀⠠⠐⠈` (8 braille frames + 1 blank rest frame), cycling at 80ms via `enable_steady_tick`.

**Theme variations:** None (identical in classic and compact).

**Configuration:** None. Characters and tick interval are hardcoded.

**Implementation:**
- Style: `running_style` in `ProgressDisplay::new()` at `src/main.rs:271-273`
- Template: `"  {spinner:.cyan}{msg} {elapsed:.dim}"`
- Tick chars: `.tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")`
- Template overhead constant: `SPINNER_TEMPLATE_OVERHEAD` in `src/theme.rs:15` (7 columns)

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
- Classic: `ClassicTheme::status_icon()` at `src/theme.rs:193-201`
- Compact: `CompactTheme::status_icon()` at `src/theme.rs:208-216`
- Alignment padding: `StepLineTheme::icon_column_width()` at `src/theme.rs:64-70`

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

**Implementation:** `StepLineTheme::render_separator()` in `src/theme.rs:159-186`. Uses `SPINNER_TEMPLATE_OVERHEAD` (7) to subtract the indicatif template's fixed-width elements for running steps.

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
- Finished: `StepLineTheme::format_elapsed()` in `src/theme.rs:58-60`
- Running: `{elapsed:.dim}` in the spinner template at `src/main.rs:271`

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

**Stderr capture:** Last 5 lines of stderr, hardcoded in `src/main.rs:441`.

**Theme variations:** Gutter width differs because `icon_column_width()` varies (classic: 5 spaces `"     "`, compact: 4 spaces `"    "`).

**Configuration:** `output.show_error_detail` (boolean, default `true`). When `false`, the error box is suppressed entirely.

**Implementation:** `StepLineTheme::render_error_detail()` in `src/theme.rs:87-110`. Error detail toggle check at `src/main.rs:436-438`.

---

## 8. Summary Separator

**What:** Visual break between the last step line and the summary.

**Visual example:**
```text
  ✅ cargo test --all-targets ........................... 0.80s
                                                              <-- blank line
Done in 23.62s
```

**Theme variations:** None (default returns empty string, rendered as a blank line).

**Configuration:** None. Overridable per theme via `render_summary_separator()`.

**Implementation:** `StepLineTheme::render_summary_separator()` in `src/theme.rs:76-78`. Called from `ProgressDisplay::on_run_finished()` at `src/main.rs:471`.

---

## 9. Summary Line

**What:** Final line showing total elapsed time and overall result.

**Visual examples:**
```text
Done in 23.62s       (all steps succeeded)
Failed in 0.42s      (one or more steps failed)
```

**Format:** `"Done in {:.2}s"` or `"Failed in {:.2}s"` — two decimal places.

**Theme variations:** None.

**Configuration:** None.

**Implementation:** `ProgressDisplay::on_run_finished()` in `src/main.rs:492-495`.

---

## Full Output Example

A complete classic-theme run with all components annotated:

```text
                                          # Component
Running: build, clippy, test              # [1] Plan Header

  ⠁ cargo build --all-targets ...... 2s   # [3] Spinner + [5] Dots + [6] Timer (running)
  ○  cargo clippy -- -D warnings ......   # [2] Pending Step Line + [4] Icon (○)
  ○  cargo test --all-targets .........   # [2] Pending Step Line

  ✅ cargo build --all-targets ...... 12.35s  # [4] Icon (✅) + [5] Dots + [6] Timer
  ✅ cargo clippy -- -D warnings .... 3.20s
  ✗  cargo test --all-targets ....... 0.42s   # [4] Icon (✗)
     ╭─                                       # [7] Error Detail Box
     │ exit status: 101                       #
     │ stderr (last 2 lines):                 #
     │   error: test failed                   #
     │   error: could not compile             #
     ╰─                                       #
                                              # [8] Summary Separator
Failed in 15.97s                              # [9] Summary Line
```

---

## TTY vs Non-TTY Rendering

| Aspect | TTY (terminal) | Non-TTY (CI / piped) |
|--------|---------------|----------------------|
| Progress bars | `MultiProgress` with `ProgressDrawTarget::stderr()` | `ProgressDrawTarget::hidden()` |
| Spinner animation | Visible, 80ms tick | Not rendered |
| Step updates | In-place redraws via indicatif | `writeln!` to stderr on each step finish/fail |
| Error detail | Inserted as progress bars after the failed step | `writeln!` to stderr |
| Summary | Progress bar via `finish_with_message` | `writeln!` to stderr |

**Implementation:** TTY detection at `src/main.rs:262`. Non-TTY fallback in `ProgressDisplay::emit_line()` at `src/main.rs:309` and `finish_bar()`.

---

## Configuration Reference

All `[output]` knobs and the components they affect:

| Config key | Type | Default | Components affected |
|-----------|------|---------|---------------------|
| `output.theme` | `"classic"` \| `"compact"` | `"classic"` | [4] Step Status Icons |
| `output.columns` | `u16` | `80` | [2] Pending Step Line, [5] Separator Dots |
| `output.show_error_detail` | `bool` | `true` | [7] Error Detail Box |

**Config sources** (later overrides earlier): embedded default → global `~/.config/cargo-ops/config.toml` → local `.ops.toml` → environment `CARGO_OPS_*`.

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
