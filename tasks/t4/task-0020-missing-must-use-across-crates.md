---
id: TASK-0020
title: "Missing #[must_use] on Result/builder methods across crates"
status: Triage
assignee: []
created_date: '2026-04-09 19:25:00'
labels: [rust-code-quality, CQ, API-5, medium]
dependencies: []
---

## Description

**Location**: multiple files
**Anchor**: various functions (see list below)
**Impact**: Several functions return `Result`, `Option`, `Vec`, or builder-pattern `Self` that callers could silently discard. The highest-risk omission is `Context::with_refresh` (builder method where discarding is a silent no-op footgun).

**Notes**:
Functions missing `#[must_use]`:

**crate-extension** (`crates/extension/src/lib.rs`):
- `Context::with_refresh` (line 286) — builder, silent discard is a bug
- `DataRegistry::schemas` (line 223) — returns `Vec`, always consumed
- `DataRegistry::provider_names` (line 216) — returns `Vec`

**crate-runner** (`crates/runner/src/command/mod.rs`):
- `resolve` (line 164) — `Option<&CommandSpec>`
- `expand_to_leaves` (line 237) — `Option<Vec<CommandId>>`
- `run_plan` (line 319) — `Vec<StepResult>`
- `run_plan_parallel` (line 448) — `Vec<StepResult>`
- `run` (line 475) — `anyhow::Result<Vec<StepResult>>`
- `list_command_ids` (line 217) — `Vec<CommandId>`

**crate-cli** (`crates/cli/src/tools_cmd.rs`):
- `run_tools_check` (line 59) — `anyhow::Result<ExitCode>`
- `run_tools_install` (line 88) — `anyhow::Result<ExitCode>`
