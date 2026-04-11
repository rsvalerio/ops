---
id: TASK-024
title: "display.rs is a 1052-line god module mixing rendering, state, and error display"
status: To Do
assignee: []
created_date: '2026-04-08 00:00:00'
labels: [rust-code-quality, CQ, ARCH-1, medium, effort-M, crate-runner]
dependencies: []
---

## Description

**Location**: `crates/runner/src/display.rs:1-1052`
**Anchor**: `struct ProgressDisplay`, `struct ErrorDetailRenderer`, `struct RenderConfig`
**Impact**: display.rs at 1052 lines mixes four distinct concerns: rendering configuration (RenderConfig), error detail rendering (ErrorDetailRenderer), progress display state management (ProgressDisplay with bars, steps, event routing, footer/separator management, TTY vs non-TTY output paths), and a 450+ line test suite. This exceeds the ARCH-1 threshold of 500 lines and mixes unrelated concerns.

**Notes**:
Code comments at lines 80-104 already acknowledge the issue and suggest splitting into ProgressState, EventDispatcher, and ErrorRenderer. Suggested split:
- `display/render.rs` — RenderConfig, theme integration
- `display/error.rs` — ErrorDetailRenderer
- `display/progress.rs` — ProgressDisplay facade
- `display/tests.rs` — test suite

The `new_with_tty_check` function (59 lines, lines 133-191) also exceeds the FN-1 threshold and would naturally be addressed by this refactoring.
