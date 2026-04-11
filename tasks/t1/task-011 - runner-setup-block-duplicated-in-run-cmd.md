---
id: TASK-011
title: "Runner setup block (config + verbose + extensions) duplicated in run_cmd.rs"
status: To Do
assignee: []
created_date: '2026-04-07 00:00:00'
labels: [rust-code-duplication, CD, DUP-1, DUP-5, low, effort-S, crate-cli]
dependencies: []
---

## Description

**Location**: `crates/cli/src/run_cmd.rs:31-36` and `crates/cli/src/run_cmd.rs:125-130`
**Anchor**: `fn run_commands`, `fn run_command`
**Impact**: A 5-line setup block is duplicated verbatim between `run_commands` and `run_command`:

**Notes**:
Both functions open with identical lines:
```rust
let (mut config, cwd) = crate::load_config_and_cwd()?;
if verbose {
    config.output.stderr_tail_lines = usize::MAX;
}
let mut runner = ops_runner::command::CommandRunner::new(config, cwd);
setup_extensions(&mut runner)?;
```

Fix option: extract a helper like `fn build_runner(verbose: bool) -> anyhow::Result<CommandRunner>` that encapsulates config loading, verbose adjustment, runner creation, and extension setup. The two callers diverge after this setup (single vs multi-command execution), so the helper returns the constructed runner.

Severity is low because the duplication is within a single file and both functions are likely to evolve together.
