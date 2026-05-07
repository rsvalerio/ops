---
id: TASK-1035
title: 'PERF-3: ConfigurableTheme::left_pad_str re-allocates on every render call'
status: Done
assignee: []
created_date: '2026-05-07 20:24'
updated_date: '2026-05-07 23:14'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/theme/src/configurable.rs:55-61` (definition); call sites at lines 128, 153, 179, 357, 389, plus internal duplicates `" ".repeat(self.left_pad())` at lines 253.

**What**: `left_pad_str()` builds a fresh `String` via `" ".repeat(self.left_pad())` on every invocation. The function is called from every theme render path:

- `render_plan_header` — once per plan (cheap)
- `render_summary_separator` — once per separator
- `render_error_detail` — once per failed step
- `render_summary` — once per run
- `render` — every step line emits at least one allocation through this helper (`pad = self.left_pad_str()` at line 357)
- `wrap_step_line` (line 253) — independently calls `" ".repeat(self.left_pad())`, missing the helper entirely
- `render_separator` indirectly through callers

`self.left_pad()` returns a constant set at theme construction (`config.left_pad`). The string is the same for the lifetime of the theme.

**Why it matters**: This mirrors exactly the pattern fixed by TASK-0747 (precompute SGR prefixes at construction). Every step line in a long-running `ops verify` (dozens to hundreds of steps) does at least one short-lived `String` allocation that is byte-identical across the entire process — pure waste. Boxed-layout themes additionally call into `inject_gutter_indent` / `right_pad_with_border` on every error-block line.

Refactor: precompute `left_pad_str: String` (or `Box<str>`/`Arc<str>`) in `ConfigurableTheme::new` alongside the SGR prefixes, expose `left_pad_str(&self) -> &str`, and replace the bare `" ".repeat(self.left_pad())` at line 253 with the cached value.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Cache the left-pad string at `ConfigurableTheme::new` time and return `&str` from `left_pad_str`
- [ ] #2 Replace the inline `" ".repeat(self.left_pad())` in `wrap_step_line` with the cached accessor
- [ ] #3 Existing render-output assertions in `crates/theme/src/tests/` continue to pass byte-for-byte
<!-- AC:END -->
