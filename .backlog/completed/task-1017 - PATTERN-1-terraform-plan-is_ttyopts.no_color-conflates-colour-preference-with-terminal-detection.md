---
id: TASK-1017
title: >-
  PATTERN-1: terraform plan is_tty=!opts.no_color conflates colour preference
  with terminal detection
status: Done
assignee: []
created_date: '2026-05-07 20:21'
updated_date: '2026-05-08 06:29'
labels:
  - code-review-rust
  - pattern
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/plan/src/lib.rs:53`

**What**: `run_plan_pipeline_to` derives `is_tty = !opts.no_color` and threads that boolean into `render_resource_table`, `render_summary_table`, `render_outputs_table`. The flag is named after a TTY but actually carries the user's colour preference. Two real failure modes follow:

1. Output piped to a file with no `--no-color`: `is_tty=true`, so `render_resource_table` calls `terminal_size::terminal_size()` (resolved against the parent process TTY) and produces a width-truncated module column that is environment-sensitive — exactly the regression ARCH-2 / TASK-0849 fixed for the `render_resource_table(.., false)` path on the *render* side. The fix on the render side relies on callers passing `false` for non-TTY, but the only caller now passes `!no_color`.
2. Output to a real TTY with `--no-color`: `is_tty=false`, terminal width is never probed and the module column is no longer right-sized.

The `comfy_table::Color` calls inside `Action::color()` are the actual concern of `--no-color`; that flag should toggle colour, not terminal-size probing. A separate `is_tty` (e.g. `std::io::stdout().is_terminal()`) should drive width handling.

**Why it matters**: ARCH-2 hardening on the render side is undone at the call site. Snapshot reproducibility comes back as soon as someone pipes plan output without `--no-color` (CI step that captures stdout, dry-run wrapper, library consumer using the new `run_plan_pipeline_to`). Operators using `--no-color` on a real TTY also lose width-aware rendering for unrelated reasons.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Decouple --no-color from TTY detection: pass a separate is_tty derived from is_terminal on the writer (or a caller-supplied flag) and use no_color only to gate Action::color() output
- [x] #2 Add a regression test: piped output (Vec<u8>) with no_color=false produces byte-identical output regardless of host terminal width
<!-- AC:END -->
