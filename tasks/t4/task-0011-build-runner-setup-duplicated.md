---
id: TASK-0011
title: "Extract build_runner() — config+runner setup duplicated in run_cmd.rs"
status: Triage
assignee: []
created_date: '2026-04-09 00:00:00'
labels: [rust-code-duplication, CD, DUP-1, DUP-5, medium, crate-cli]
dependencies: []
---

## Description

**Location**: `crates/cli/src/run_cmd.rs:31-36` and `crates/cli/src/run_cmd.rs:124-130`
**Anchor**: `fn run_commands`, `fn run_command`
**Impact**: Five identical lines (config load, verbose override, runner creation, extension setup) at the top of both `run_commands()` and `run_command()`. A change to the setup sequence (e.g., adding a new extension or config flag) must be applied in both places.

**Notes**:
Duplicated block:
```rust
let (mut config, cwd) = crate::load_config_and_cwd()?;
if verbose { config.output.stderr_tail_lines = usize::MAX; }
let mut runner = ops_runner::command::CommandRunner::new(config, cwd);
setup_extensions(&mut runner)?;
```

Fix: extract `fn build_runner(verbose: bool) -> anyhow::Result<CommandRunner>` and call from both functions.
