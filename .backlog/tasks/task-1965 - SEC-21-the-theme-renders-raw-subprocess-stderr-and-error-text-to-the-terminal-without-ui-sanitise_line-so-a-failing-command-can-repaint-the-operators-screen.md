---
id: TASK-1965
title: >-
  SEC-21: the theme renders raw subprocess stderr and error text to the terminal
  without ui::sanitise_line, so a failing command can repaint the operator's
  screen
status: Done
assignee:
  - TASK-1987
created_date: '2026-08-27 15:53'
updated_date: '2026-08-28 19:30'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - crates/theme/src/render.rs
  - crates/theme/src/configurable.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/theme/src/render.rs:12-46` (`render_error_block`), `crates/theme/src/configurable.rs:196-224` (`render_error_detail`), `crates/theme/src/configurable.rs:461-470` (`render_report` detail passthrough)

**What**: `render_error_block` interpolates `detail.message` and every `detail.stderr_tail` entry straight into the output lines:

    lines.push(format!("{}{}{} {}", pad, gutter, mid, detail.message));
    for stderr_line in &detail.stderr_tail {
        lines.push(format!("{pad}{gutter}{mid}   {stderr_line}"));
    }

Those strings are verbatim child-process stderr. The producer chain is:

- `crates/runner/src/display/error_detail.rs:34-38` — `extract_stderr_tail` copies the captured `OutputLine`s with `l.as_str().to_string()`, no filtering.
- `crates/runner/src/display/error_detail.rs:23-24` — wraps them in `ErrorDetail::new` and calls `theme.render_error_detail`.
- `crates/runner/src/display.rs:633-637` — `render_error_details_non_tty` hands each returned line to `write_stderr`, which is a bare `writeln!(io::stderr(), "{text}")` (display.rs:45-53). On the TTY path they go to `ProgressBar::finish_with_message`, which also does not filter.

Nothing on that path calls `ops_core::ui::sanitise_line`, the project's own SEC-21 defence (crates/core/src/ui.rs:30-40) that escapes ESC and every non-tab control byte. `ui::error` / `ui::warn` / the `--dry-run` audit channel / `project_identity::card` all route through it; the theme error block does not. So any tool whose stderr contains attacker- or dependency-controlled text (a test name, a compiler note quoting a source string, a fetched package's banner) can emit ESC sequences that clear the screen, move the cursor, set the window title, or open an OSC-8 hyperlink in the operator's terminal.

**Second-order effect**: the same bytes corrupt this crate's own layout. In `render_error_detail` the boxed path measures each line with `visible_width`, which treats an ESC sequence as zero columns, then right-pads to `right_target` and appends ` │`. The terminal, however, *acts* on the sequence, so the closing frame bar lands in the wrong column and the box breaks for the rest of the run. A bare `\r` in a stderr line is worse: `visible_width` counts it as 0 and the terminal returns to column 0, overwriting the frame entirely.

**Why it matters**: this is the crate's largest untrusted-input surface — everything else it renders comes from config, this comes from arbitrary child processes. TASK-1843 (ops-core `sanitise_line` C1/bidi passthrough) narrows the defence; this finding is that the defence is not applied here at all.

**Note on scope**: `render_report`'s flat path also pushes `row.details` verbatim (`out.push(detail.clone())`), and `render_report_boxed` passes them to `wrap_box_content` — same class, same fix point.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 detail.message, every detail.stderr_tail entry, and every report row detail line are routed through ops_core::ui::sanitise_line (or an equivalent escaping helper) before being interpolated into a rendered line
- [ ] #2 a test renders an ErrorDetail whose stderr_tail contains ESC[2J, a bare CR, and an OSC-8 hyperlink, and asserts the returned lines contain no 0x1b, 0x0d or other C0 byte
- [ ] #3 a test asserts that a boxed-layout error line whose stderr text contains an ESC sequence still has visible_width equal to the frame width and still ends with the closing bar
<!-- AC:END -->
