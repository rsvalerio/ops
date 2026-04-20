---
id: TASK-0110
title: >-
  ARCH-1: crates/runner/src/display.rs is ~1350 lines mixing render, TAP, event
  routing, plan state
status: Done
assignee: []
created_date: '2026-04-19 18:36'
updated_date: '2026-04-19 19:12'
labels:
  - rust-code-review
  - architecture
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/display.rs` (1350 lines)

**What**: `ProgressDisplay` and its free functions mix: indicatif bar management, theme-driven step rendering, stderr capture, box/summary rendering, plan-lifecycle state, and TAP logging in a single file.

**Why it matters**: ARCH-1 red flags (>500 lines, multiple unrelated concerns, wide public surface). Prior task TASK-0064 only addressed the single `new_with_tty_check` constructor; the module-level cohesion issue remains.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Extract at least one cohesive concern into its own submodule (e.g., TAP logger, box/summary renderer, or plan state tracker)
- [x] #2 crates/runner/src/display.rs drops below 800 lines or clearly becomes a thin orchestrator
- [x] #3 cargo fmt; cargo clippy --all-targets -- -D warnings; cargo test all pass
<!-- AC:END -->
