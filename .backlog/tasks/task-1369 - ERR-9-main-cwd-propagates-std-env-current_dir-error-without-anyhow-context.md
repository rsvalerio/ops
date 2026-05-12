---
id: TASK-1369
title: >-
  ERR-9: main::cwd() propagates std::env::current_dir() error without anyhow
  context
status: Done
assignee:
  - TASK-1386
created_date: '2026-05-12 21:34'
updated_date: '2026-05-12 23:45'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/main.rs:278-280`

**What**: `pub(crate) fn cwd() -> anyhow::Result<PathBuf> { Ok(std::env::current_dir()?) }` propagates the raw `std::io::Error` from `current_dir()` through the `?` operator with no `.with_context(...)`. When the syscall fails (deleted cwd, permission denied on a parent component, very long path) the user sees a bare `No such file or directory (os error 2)` with no indication that `ops` was trying to resolve the working directory.

**Why it matters**: ERR-9 — `.with_context()` is the standard idiom for anyhow error propagation. The function name `cwd()` is the only signal as to what was happening; the user-facing message strips that. Compare with `run_with_runtime_kind` in run_cmd.rs which already does `.context("failed to start tokio runtime for command execution")?` — the same discipline should apply here. This helper is called from `new_command_cmd::run_new_command` (via `dispatch`), `extension_cmd::run_extension_list_to`, `extension_cmd::run_extension_show_with_tty_check`, `subcommands::cli_data_context`, `subcommands::run_theme::Select`, and `run_cmd::build_runner` — every CLI subcommand surfaces the bare OS error.

Sibling site `hook_shared::run_hook_install` (line 45) has the same un-contexted `std::env::current_dir()?` pattern. TASK-1347 already covers hook_shared's cwd error context — this is the broader main::cwd() helper that other handlers route through.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 main::cwd() attaches an anyhow context such as 'failed to read current working directory' before propagating the io::Error
- [x] #2 the user-facing error from a failing current_dir() includes the context string, not just the bare OS error
<!-- AC:END -->
