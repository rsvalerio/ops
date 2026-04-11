---
id: TASK-0008
title: "Extract shared is_stdout_tty() — identical 3-line function in 3 modules"
status: Triage
assignee: []
created_date: '2026-04-09 00:00:00'
labels: [rust-code-duplication, CD, DUP-1, medium, crate-cli]
dependencies: []
---

## Description

**Location**: `crates/cli/src/extension_cmd.rs:138-140`, `crates/cli/src/theme_cmd.rs:87-89`, `crates/cli/src/new_command_cmd.rs:9-11`
**Anchor**: `fn is_stdout_tty`
**Impact**: Three identical copies of a trivial function. If TTY detection logic ever needs to change (e.g., respecting `--no-tty` flag or `NO_COLOR`), all three must be updated in lockstep.

**Notes**:
Each copy is:
```rust
fn is_stdout_tty() -> bool {
    io::stdout().is_terminal()
}
```
Extract to a shared location (e.g., `crate::tty::is_stdout_tty()` or a top-level `mod util`) and import from each module.
