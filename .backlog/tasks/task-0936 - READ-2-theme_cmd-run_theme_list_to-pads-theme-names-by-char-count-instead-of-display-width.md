---
id: TASK-0936
title: >-
  READ-2: theme_cmd run_theme_list_to pads theme names by char count instead of
  display width
status: Done
assignee: []
created_date: '2026-05-02 15:50'
updated_date: '2026-05-02 17:26'
labels:
  - code-review-rust
  - READ
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/theme_cmd.rs:66-89`

**What**: `run_theme_list_to` computes `max_name_len = options.iter().map(|o| o.name.len()).max()` (byte length) and then renders each row with `"{:width$}"` (Rust's char-count padding). Both measurements diverge from display width, so a wide-character theme name (CJK ideograph, emoji, combining mark) skews column alignment.

This is the same bug pattern that TASK-0758 fixed for `tools_cmd::run_tools_list_to` and TASK-0734 fixed for `help::render_grouped_sections`. The shared `ops_core::output::display_width` helper plus a manual space-pad loop is the established workaround.

Theme names are user-supplied (`[themes.<name>]` is unrestricted in `.ops.toml`), so this is reachable in practice the moment a config defines a theme like `[themes.ビルド]` or `[themes.🚀classic]`.

**Why it matters**: Operator-facing list output mis-aligns the description column on wide-char theme names, regressing the multibyte-safe alignment work that already shipped for the parallel `tools` and `help` paths.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 run_theme_list_to measures theme name width via display_width, not str::len
- [x] #2 Padding is emitted via manual space-pad (mirroring tools_cmd / help.rs), not the Rust {:width$} format spec
- [ ] #3 Regression test asserts that a wide-char theme name aligns the description column at the same display column as an ASCII theme name
<!-- AC:END -->
