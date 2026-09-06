---
id: TASK-1976
title: >-
  CL-3: apply_style documents stderr gating but color_enabled ORs stdout, so
  piping only stderr to a file fills it with SGR codes
status: Done
assignee:
  - TASK-1987
created_date: '2026-08-27 15:55'
updated_date: '2026-08-28 19:29'
labels:
  - code-review-rust
  - idioms
dependencies: []
modified_files:
  - crates/theme/src/style/sgr.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/theme/src/style/sgr.rs:13-38` (`apply_style`, `color_enabled`), `crates/theme/src/style/sgr.rs:75-84` (`apply_with_prefix`)

**What**: the doc comment on `apply_style` states the contract as

    /// Wrap `text` in ANSI SGR codes derived from `spec`, if stderr is a TTY
    /// (and `NO_COLOR` is unset) ...

but the resolver it delegates to is

    fn color_enabled() -> bool { ops_core::style::color_enabled() }

and that is `(stdout_is_terminal() || stderr_is_terminal()) && !no_color_env()` (crates/core/src/style.rs:25-27). Everything this crate renders goes to stderr — the runner writes step lines and error blocks through `write_stderr` and indicatif's stderr-backed `MultiProgress` (crates/runner/src/display.rs:45-53). So `ops verify 2> build.log` from an interactive shell leaves stdout a TTY, `color_enabled()` returns true, and the log file receives escape codes for output that no terminal will ever render. The reverse case is the same defect mirrored: `ops verify > out.txt` with stderr still on the terminal is correctly colored only by accident of the OR.

The cross-crate cause is `ops_core::style::color_enabled`, whose OR was chosen for the shared table renderer that writes to stdout. TASK-1188 deliberately routed this crate onto that shared resolver to stop the two subsystems disagreeing, and in doing so replaced a stderr-specific check with a stream-agnostic one without updating the documented contract here. The fix belongs in ops-theme: this crate knows which stream it renders to and should gate on that stream rather than on "either stream".

**Why it matters**: CI logs and redirected build logs are the normal consumers of this output, and escape codes in them are noise that also defeats grep. The mismatch between the stated contract and the behaviour is what makes it hard to notice: a reader of `apply_style` has no reason to suspect stdout is involved. `apply_with_prefix` (the hot path used by `render_slot`, `render_summary_text` and both border builders) has the same gate.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 the theme colour gate is resolved against the stream this crate actually renders to (stderr), not the logical OR of stdout and stderr, while still honouring NO_COLOR
- [ ] #2 the apply_style and apply_with_prefix doc comments state the gate that the code implements
- [ ] #3 a test pins that with stderr redirected and stdout a TTY the rendered step line contains no 0x1b byte
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
AC#3 substitution: a test harness cannot give a process a real TTY on stdout, so "stdout a TTY, stderr redirected" is pinned in two halves that together cover the scenario — `gate_ignores_stdout_and_follows_stderr` asks the pure resolver about exactly that stream combination (theme gate false while ops-core's OR-gate is true), and `step_line_is_plain_when_stderr_is_redirected` renders a real step line in this process, where stderr is redirected, and asserts it carries no 0x1b.
<!-- SECTION:NOTES:END -->
