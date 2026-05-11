---
id: TASK-1275
title: >-
  SEC-21: Dry-run prints command name and leaf id without audit_safe
  sanitisation
status: To Do
assignee:
  - TASK-1305
created_date: '2026-05-11 15:25'
updated_date: '2026-05-11 16:48'
labels:
  - code-review-rust
  - security
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/run_cmd/dry_run.rs:42-46`

**What**: `run_command_dry_run_to` writes the user-supplied `name` and the resolved leaf `id` directly via `writeln!`. Both can originate from `.ops.toml` command keys (TOML quoted keys allow arbitrary Unicode, including ESC/NUL/CR), but only program/args/env/cwd values are routed through `audit_safe()`. An attacker-controlled `.ops.toml` key like `"build[2J"` would emit raw ANSI to stdout during `ops --dry-run`.

**Why it matters**: Breaks the documented SEC-21 contract that audit-channel output cannot repaint or clobber the operator's terminal, undermining defence-in-depth that the rest of `print_exec_spec` already enforces.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Both name and each id are written through audit_safe() (or equivalent sanitise_line wrapper)
- [ ] #2 Add a unit test analogous to dry_run_escapes_ansi_in_program_args_env_and_cwd covering a command name/id containing ESC bytes
- [ ] #3 No remaining writeln!() in dry_run.rs that emits caller- or config-controlled strings unsanitised
<!-- AC:END -->
