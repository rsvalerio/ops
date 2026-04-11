---
id: TASK-025
title: "display.rs on_run_finished has 5-level nesting with 5+ boolean paths"
status: To Do
assignee: []
created_date: '2026-04-08 00:00:00'
labels: [rust-code-quality, CQ, FN-2, FN-5, medium, effort-S, crate-runner]
dependencies: []
---

## Description

**Location**: `crates/runner/src/display.rs:467-518`
**Anchor**: `fn on_run_finished`
**Impact**: on_run_finished has 5 levels of nesting and 5+ distinct boolean paths: total_steps > 0, success flag, is_tty check, footer_bar presence, separator emptiness, and write error handling. This exceeds both the FN-2 nesting threshold (≤4) and creates high cognitive load.

**Notes**:
The function mixes summary rendering with TTY-specific separator/footer logic. Refactoring approach:
- Extract TTY footer rendering into a `render_footer()` helper
- Extract summary line formatting into `format_summary_line()`
- Use early returns and guard clauses to flatten the nesting
