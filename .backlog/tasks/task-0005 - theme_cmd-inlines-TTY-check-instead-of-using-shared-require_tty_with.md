---
id: TASK-0005
title: theme_cmd inlines TTY check instead of using shared require_tty_with
status: Done
assignee: []
created_date: '2026-04-10 14:30:00'
updated_date: '2026-04-11 09:55'
labels:
  - rust-code-duplication
  - CD
  - DUP-3
  - low
  - crate-cli
dependencies: []
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Location**: `crates/cli/src/theme_cmd.rs:105-111`
**Anchor**: `fn run_theme_select_with_tty_check`
**Impact**: `run_theme_select_with_tty_check` does its own inline TTY check (`if !is_tty() { anyhow::bail!("theme select requires an interactive terminal") }`) instead of calling `crate::tty::require_tty_with("theme select", is_tty)`, which is the pattern used by every other interactive command (`new_command_cmd.rs`, `run_before_commit_cmd.rs`, `run_before_push_cmd.rs`, `run_before_commit_cmd.rs`). This inconsistency means the TTY error message format and behavior drifts from the established convention.

**Notes**:
The function already accepts an `is_tty: F` closure for testability — it just doesn't delegate to the shared utility. Fix is a one-line replacement: replace lines 109–111 with `crate::tty::require_tty_with("theme select", is_tty)?;`. The test `run_theme_select_non_tty_returns_error` should continue to pass since `require_tty_with` produces the same error pattern.

DUP-3: repeated pattern (TTY check) that already has an extracted helper but isn't used consistently.
<!-- SECTION:DESCRIPTION:END -->
