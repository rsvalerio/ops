---
id: TASK-1979
title: >-
  TEST-25: every error-block colour test asserts against a local
  re-implementation of render_error_block that has already drifted from it
status: Triage
assignee: []
created_date: '2026-08-27 15:56'
labels:
  - code-review-rust
  - test-quality
dependencies: []
modified_files:
  - crates/theme/src/tests/error_block_color.rs
  - crates/theme/src/render.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/theme/src/tests/error_block_color.rs:10-27` (`render_with` helper), used by all three tests in the file (lines 30-85)

**What**: the helper does not call the function under test. It rebuilds it:

    fn render_with(chars: &ErrorBlockChars, enabled: bool) -> Vec<String> {
        // Mirror render_error_block's structure but with explicit styling gate,
        let detail = ErrorDetail::new("exit status: 1".to_string(), vec![]);
        let pad = String::new();
        let gutter = if chars.rail.is_empty() { "    ".to_string() } else { format!("{}   ", chars.rail) };
        ...
    }

`render_error_block` (crates/theme/src/render.rs:12-46) is never invoked. So `error_block_color_wraps_top_mid_bottom_with_sgr_when_enabled`, `error_block_rail_remains_unstyled_when_color_set`, and `error_block_unknown_color_does_not_change_display_width` all pass or fail on the copy, not on production. The three properties they claim to pin — that top/mid/bottom carry SGR, that the rail does not, and that an unrecognised colour spec is width-neutral — could all be broken in `render_error_block` with the suite still green.

The copy has already drifted:

- `gutter` for the empty-rail case is the literal `"    "` (4 spaces); production computes `" ".repeat(icon_column_width.saturating_add(3))`, which is 5 for the compact theme (icon column width 2) and varies per theme.
- `pad` is a fixed `String::new()`; production applies `" ".repeat(left_pad)`.
- The copy always emits exactly three lines; production has an early return for the empty-detail case and a whole `stderr_tail` branch (the `stderr (last N lines):` header plus one line per entry) that no colour test touches.

The stated reason for the copy — that `apply_style` consults live TTY state — is real but has a cheaper fix than duplicating the function: the crate already exports `apply_style_gated`, and the tests already use `EnvGuard`/`#[serial]` elsewhere (crates/theme/src/tests/mod.rs, render_basics.rs) to force `NO_COLOR`.

**Why it matters**: these are the only tests covering the colour behaviour of the error block, and they cover a function that does not ship.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 the three error-block colour tests call render_error_block (or ConfigurableTheme::render_error_detail) instead of a local re-implementation
- [ ] #2 the TTY dependency is handled by an injected gate on the production path or by an EnvGuard NO_COLOR serial test, not by duplicating the rendering logic
- [ ] #3 the gutter width assertion is derived from icon_column_width rather than a hardcoded four spaces, so a theme with wider icons is covered
- [ ] #4 at least one colour test exercises the stderr_tail branch of render_error_block, which no current test in this file reaches
<!-- AC:END -->
