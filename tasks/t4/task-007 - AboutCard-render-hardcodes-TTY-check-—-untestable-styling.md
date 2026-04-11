---
id: TASK-007
title: 'AboutCard::render hardcodes TTY check — untestable styling'
status: To Do
assignee: []
created_date: '2026-04-07 00:00:00'
updated_date: '2026-04-07 22:48'
labels:
  - rust-code-quality
  - CQ
  - FN-9
  - READ-1
  - medium
  - effort-S
  - crate-core
dependencies: []
ordinal: 6000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Location**: `crates/core/src/project_identity.rs:119`
**Anchor**: `fn render`
**Impact**: `AboutCard::render` calls `io::stdout().is_terminal()` directly at line 119, embedding implicit state into the function. This makes it impossible to test styled vs unstyled output without controlling the actual terminal state. FN-9 requires explicit dependencies (no implicit state). The same anti-pattern was identified in CLI commands (TASK-002 covers the `is_stdout_tty()` duplication), but this instance is in `core` and affects the about card rendering path.

**Notes**:
Fix: accept `is_tty: bool` as a parameter, following the pattern already used by `ProgressDisplay::new_with_tty_check` in the runner crate:

```rust
pub fn render(&self, _columns: u16, is_tty: bool) -> String {
    // ...use is_tty instead of io::stdout().is_terminal()
}
```

The caller (`ops_about::run_about`) already knows TTY state and can pass it through. This enables testing both styled and plain output paths.
<!-- SECTION:DESCRIPTION:END -->
