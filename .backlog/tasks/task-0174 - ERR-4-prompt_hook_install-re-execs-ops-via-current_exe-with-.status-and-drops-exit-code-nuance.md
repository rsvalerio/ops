---
id: TASK-0174
title: >-
  ERR-4: prompt_hook_install re-execs ops via current_exe() with .status()? and
  drops exit code nuance
status: Done
assignee: []
created_date: '2026-04-22 21:25'
updated_date: '2026-04-23 07:43'
labels:
  - rust-code-review
  - ERR
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/subcommands.rs:80-89`

**What**: `prompt_hook_install` calls `std::process::Command::new(std::env::current_exe()?).args([hook_name, "install"]).status()?` — re-executing the current `ops` binary as a subprocess to run the install flow, instead of calling `run_before_commit_cmd::run_before_commit_install()` directly. This doubles the process count, loses ExitCode fidelity (only Success/Failure, not the real code), and swallows the Err side of `.status()?` into `anyhow` without context.

**Why it matters**: ERR-4 + ARCH-2. The install functions are in the same crate (`run_before_commit_cmd`, `run_before_push_cmd`) and already return `anyhow::Result<()>` — no need to re-exec. Fix: match on `hook_name` and dispatch to the in-process installer. If there is a specific reason the current design uses re-exec (e.g. clearing state, reloading config), document it on the function.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 prompt_hook_install dispatches in-process instead of re-execing current_exe
<!-- AC:END -->
