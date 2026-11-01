---
id: TASK-2651
title: 'ops config check reads only mise.toml''s bare ops key'
status: Done
assignee: []
created_date: '2026-11-01 00:41'
updated_date: '2026-11-01 13:45'
labels:
  - code-review-rust
  - config
  - ci
dependencies: []
parent_task_id: 'TASK-2655'
modified_files:
  - crates/core/src/config/history.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/history.rs:1` (`mise_ops_pin`)

**What**: The pin is read from `tools.ops` in `mise.toml` only. mise also reads `.mise.toml` and `.config/mise.toml`, and a project can pin the tool under its backend key `"github:rsvalerio/ops"` with no `[tool_alias]`.

**Why it matters**: In those layouts `ops config check` finds no pin, prints only the minimum and exits 0, so a real pin mismatch passes. `ops init --rust --check` shares the lookup.

**Origin**: discovered during TASK-2646 while fixing TASK-2638.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Every pin location mise honours for ops is read, or the supported set is documented

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
mise_ops_pin now reads every committed single-file mise location mise honours at the project root - mise.toml, .mise.toml, mise/config.toml, .mise/config.toml, .config/mise.toml, .config/mise/config.toml - in mise's precedence order (last file naming a pin wins, as in mise), under both the ops key and the backend key github:rsvalerio/ops. The winning file is reported (new MiseOpsPin struct; ops config check prints '<file> pin', foundation drift names the file). Deliberately not read, documented in docs/configuration.md and the MISE_FILES doc comment: *.local.toml (uncommitted, CI never installs it), conf.d directories, parent directories, the user's global config and .tool-versions. Ripple: crates/core/src/config/mod.rs re-export and crates/cli/src/config_cmd.rs source label.
<!-- SECTION:NOTES:END -->
