---
id: TASK-1746
title: >-
  READ-6: inquire cancellation is handled three different ways; theme select /
  extension show / new-command surface Esc as 'ops: error:' and exit 1
status: To Do
assignee:
  - TASK-1982
created_date: '2026-08-27 11:13'
updated_date: '2026-08-28 14:08'
labels:
  - code-review-rust
  - readability
dependencies: []
modified_files:
  - crates/cli/src/theme_cmd.rs
  - crates/cli/src/extension_cmd.rs
  - crates/cli/src/new_command_cmd.rs
  - crates/cli/src/about_cmd.rs
  - crates/cli/src/subcommands.rs
  - crates/cli/src/hook_shared.rs
  - crates/cli/src/import_makefile_cmd.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/theme_cmd.rs:166-168`, `crates/cli/src/extension_cmd.rs:311`, `crates/cli/src/new_command_cmd.rs:20-42`

**What**: The crate has a deliberate, documented convention for "the user pressed Esc / Ctrl-C at a prompt": it is a cancel, not a failure, and it exits with `SIGINT_EXIT` (130) without the `ops: error:` frame that `main` wraps around anyhow errors. Three implementations of that convention already exist, each spelled differently:

1. `hook_shared.rs:139-145` — matches `OperationCanceled | OperationInterrupted` and returns `Err(anyhow!("... cancelled").context(ExitCodeOverride(SIGINT_EXIT)))`, which `main::extract_exit_code_override` unwraps.
2. `subcommands.rs:99-112` — `classify_confirm_result` maps the same two variants to `Ok(None)`; `prompt_hook_install` then returns `Ok(ExitCode::from(SIGINT_EXIT))`.
3. `import_makefile_cmd.rs:141-143` — `prompt_target_selection` maps them to `Ok(None)`; the caller prints `Cancelled; .ops.toml left untouched.` and returns `ExitCode::from(SIGINT_EXIT)`.

The remaining four prompt sites do none of this and propagate the `InquireError` with a bare `?`:

- `theme_cmd.rs:168` — `inquire::Select::new("Select a theme:", options)...prompt()?`
- `extension_cmd.rs:311` — `inquire::Select::new("Select an extension:", options).prompt()?`
- `new_command_cmd.rs:22` — `inquire::Text::new("Full command:")...prompt()?`
- `new_command_cmd.rs:42` — `inquire::Text::new("Command name:")...prompt()?`

(`about_cmd.rs:651`, `inquire::MultiSelect ... .prompt()?`, is a fifth.)

So pressing Esc at `ops theme select` prints `ops: error: Operation was canceled by the user` and exits 1, while pressing Esc one prompt later in `ops run-before-commit install` exits 130 cleanly. Exit 1 is indistinguishable from a real failure, so a script or wrapper cannot tell "the user backed out" from "the command broke" — which is exactly the distinction `ExitCodeOverride`'s own doc comment in `main.rs:82-90` says the sentinel exists to preserve.

There is also no shared helper: `classify_confirm_result` is private to `subcommands.rs` and typed to `Result<bool, _>` (Confirm only), so neither Select, MultiSelect, nor Text can reuse it, and the Select/MultiSelect cancel arms in `hook_shared` and `import_makefile_cmd` are copy-pasted match arms (DUP-3, 3 occurrences of the same `OperationCanceled | OperationInterrupted` pattern).

**Why it matters**: READ-6 — consistent patterns for similar problems. Half the interactive surface honours the cancel convention and half does not, and which half a command falls into is invisible from the call site. The user-visible consequence is a spurious `ops: error:` line and a wrong exit code on the ordinary "changed my mind" path.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A single generic cancel-classification helper (generic over the prompt's Ok type, not Confirm-specific) lives in one module and is reused by every inquire callsite in the crate
- [ ] #2 theme select, extension show, new-command (both prompts) and about setup route Esc / Ctrl-C through that helper so they exit with SIGINT_EXIT (130) and no 'ops: error:' frame, instead of the current exit 1
- [ ] #3 The existing three cancel sites (hook_shared, subcommands classify_confirm_result, import_makefile prompt_target_selection) are migrated to the shared helper so the OperationCanceled|OperationInterrupted match arm appears exactly once in the crate
- [ ] #4 Non-cancel InquireError variants still propagate as anyhow errors carrying a context that names the prompt source, matching the current classify_confirm_result behaviour
- [ ] #5 A unit test per migrated command asserts that a cancelled prompt yields the 130 exit path and not an anyhow Err
<!-- AC:END -->
