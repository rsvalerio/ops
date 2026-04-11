---
id: TASK-0010
title: "Extract duplicated loop body in inject_dynamic_commands and inline command_description"
status: Triage
assignee: []
created_date: '2026-04-09 00:00:00'
labels: [rust-code-duplication, CD, DUP-1, DUP-5, medium, crate-cli]
dependencies: []
---

## Description

**Location**: `crates/cli/src/main.rs:229-242` and `crates/cli/src/main.rs:246-259`
**Anchor**: `fn inject_dynamic_commands`
**Impact**: Two near-identical 10-line loop bodies that build clap subcommands from `CommandSpec`. The description-extraction logic (`spec.help().map(...).unwrap_or_else(...)`) is also duplicated as a standalone `command_description()` in `run_before_commit_cmd.rs:74-78`.

**Notes**:
Both loops in `inject_dynamic_commands` do:
```rust
if builtins.contains(name.as_str()) || !seen.insert(name.clone()) { continue; }
let about = spec.help().map(|s| s.to_string()).unwrap_or_else(|| spec.display_cmd_fallback());
let mut sub = clap::Command::new(leak(name.clone())).about(leak(about));
for alias in spec.aliases() { sub = sub.visible_alias(leak(alias.clone())); }
cmd = cmd.subcommand(sub);
```

Fix: extract a helper (closure or fn) that takes `(cmd, name, spec, builtins, seen)` and returns the updated `cmd`. Consider adding a `CommandSpec::description()` method to eliminate the `help().map(...).unwrap_or_else(...)` pattern in both locations.
