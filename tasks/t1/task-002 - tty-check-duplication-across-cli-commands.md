---
id: TASK-002
title: "TTY check function and bail pattern duplicated across 4 CLI command modules"
status: To Do
assignee: []
created_date: '2026-04-06 00:00:00'
labels: [rust-code-duplication, CD, DUP-1, DUP-2, DUP-5, medium, effort-S, crate-cli]
dependencies: []
---

## Description

**Location**: `crates/cli/src/{theme_cmd.rs:87-89, extension_cmd.rs:138-140, new_command_cmd.rs:9-11}`
**Anchor**: `fn is_stdout_tty`, `fn run_theme_select_with_tty_check`, `fn run_new_command_with_tty_check`, `fn run_extension_show_with_tty_check`, `fn run_before_commit_install`
**Impact**: The identical `is_stdout_tty()` function is copy-pasted in 3 modules, and the TTY-check-then-bail pattern appears in 4 modules with only the command name differing.

**Notes**:
Three identical copies of:
```rust
fn is_stdout_tty() -> bool {
    io::stdout().is_terminal()
}
```
At `theme_cmd.rs:87`, `extension_cmd.rs:138`, `new_command_cmd.rs:9`.

The TTY bail pattern repeats 4 times:
- `theme_cmd.rs:114-115`: `anyhow::bail!("theme select requires an interactive terminal")`
- `new_command_cmd.rs:21-22`: `anyhow::bail!("new-command requires an interactive terminal")`
- `extension_cmd.rs:156-157`: `anyhow::bail!("extension show requires an interactive terminal (or pass a name)")`
- `run_before_commit_cmd.rs:81-82`: `anyhow::bail!("run-before-commit install requires an interactive terminal")`

Fix: extract a shared helper into a common module (e.g. `cli/src/tty.rs` or inline in `test_utils.rs`):
```rust
pub(crate) fn require_tty(cmd_name: &str) -> anyhow::Result<()> {
    if !io::stdout().is_terminal() {
        anyhow::bail!("{cmd_name} requires an interactive terminal");
    }
    Ok(())
}
```
For testability, accept an `is_tty: impl FnOnce() -> bool` parameter in the shared version, eliminating the separate `is_stdout_tty()` copies.
