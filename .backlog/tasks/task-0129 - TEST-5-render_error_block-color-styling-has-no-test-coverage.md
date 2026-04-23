---
id: TASK-0129
title: 'TEST-5: render_error_block color styling has no test coverage'
status: To Do
assignee: []
created_date: '2026-04-22 20:27'
updated_date: '2026-04-23 06:45'
labels:
  - rust-code-review
  - test
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/theme/src/render.rs:24-26` and `crates/theme/src/tests.rs`

**What**: The new `ErrorBlockChars.color` field (commit b6817e5) is applied via `apply_style` to the `top`/`mid`/`bottom` glyphs but no test asserts that:
  1. A non-empty `color` produces ANSI SGR codes wrapping `top`/`mid`/`bottom` when styling is enabled.
  2. The `rail` glyph stays unstyled (deliberately neutral so it lines up with the surrounding box border).
  3. An unknown color spec degrades to plain text without affecting layout (`display_width(strip_ansi(line))` stays the same).

**Why it matters**: This is a public-facing visual contract baked into the default `studio` theme (`crates/core/src/.default.ops.toml` now ships `color = "red dim"`). A future refactor of `render_error_block` could silently drop the styling or accidentally color the rail, breaking the boxed layout alignment without any test failing. TEST-5 (public API tests) and TEST-6 (error/edge paths) both apply.

**Suggested coverage** (use `apply_style_gated(..., enabled=true)` pattern already present in `style.rs` tests):
  - `error_block_color_wraps_top_mid_bottom_with_sgr_when_enabled`
  - `error_block_rail_remains_unstyled_when_color_set`
  - `error_block_unknown_color_does_not_change_display_width`
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 New tests in crates/theme/src/tests.rs cover the three scenarios above using apply_style_gated or equivalent TTY-gated helper
- [ ] #2 Tests assert layout invariant: display_width(strip_ansi(line)) is unchanged when color is set vs empty
- [ ] #3 Tests confirm the rail glyph in studio-style configs (rail = "│") never carries ANSI codes even when color is non-empty
<!-- AC:END -->
