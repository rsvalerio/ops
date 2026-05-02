---
id: TASK-0849
title: >-
  ARCH-2: render_resource_table queries real terminal_size, ignoring
  caller-supplied is_tty
status: Done
assignee: []
created_date: '2026-05-02 09:16'
updated_date: '2026-05-02 14:18'
labels:
  - code-review-rust
  - architecture
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/plan/src/render.rs:67`

**What**: `let term_width = terminal_size::terminal_size().map(...)` is called inside a pure render function whose is_tty argument is the only signal callers expect to control output shape. When is_tty=false (piped, tests, CI snapshots) the function still consults real stdout TTY size and produces width-dependent output.

**Why it matters**: Same class of bug already filed elsewhere in the workspace (task-0781). Makes snapshot tests environment-sensitive and breaks rendering for non-stdout sinks.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Probe terminal width only when is_tty == true; otherwise return without set_max_width
- [ ] #2 Or accept term_width: Option<usize> as an explicit parameter and let the binary supply it
- [x] #3 Add a regression test that asserts render_resource_table(..., false) is byte-identical regardless of the host terminal width
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
render_resource_table now only consults terminal_size::terminal_size() when is_tty == true; under is_tty=false term_width stays None and no set_max_width call is made. Added regression test resource_table_non_tty_output_is_stable_across_term_widths pinning byte-equal output across calls under is_tty=false. AC#2 (caller-supplied term_width) not chosen — keeping the existing is_tty signature is the smaller change and matches sibling render_summary_table / render_outputs_table.
<!-- SECTION:NOTES:END -->
