---
id: TASK-0881
title: >-
  ARCH-1: theme/style.rs mixes ANSI parsing/emission, OnceLock TTY probe, and
  ANSI stripping
status: Done
assignee: []
created_date: '2026-05-02 09:25'
updated_date: '2026-05-02 11:04'
labels:
  - code-review-rust
  - architecture
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/theme/src/style.rs:1-220`

**What**: The module owns three concerns: SGR token parsing (parse_spec/token_code), runtime TTY/NO_COLOR gating (color_enabled/no_color_env), and ANSI escape stripping (visible_width/strip_ansi). The stripping logic is reused by output consumers via ops_theme::strip_ansi, but the gate logic is rendering-private.

**Why it matters**: ARCH-1 / ARCH-4 - splitting sgr.rs (parse + apply) from strip.rs (strip/visible_width) would give a clearer cross-crate API and make visible_width callable by core::output::display_width without the rendering crate gate code.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Split into style/sgr.rs and style/strip.rs (or hoist strip_ansi/visible_width into ops_core::output)
- [ ] #2 Re-export from theme::style to keep the public API stable
- [ ] #3 Update internal imports
<!-- AC:END -->
