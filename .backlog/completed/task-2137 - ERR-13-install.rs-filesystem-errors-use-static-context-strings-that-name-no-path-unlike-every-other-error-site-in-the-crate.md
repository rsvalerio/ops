---
id: TASK-2137
title: 'ERR-13: install.rs filesystem errors use static context strings that name no path, unlike every other error site in the crate'
status: Done
assignee: []
created_date: '2026-09-08 06:56'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-rust
  - error-handling
dependencies: []
parent_task_id: 'TASK-2249'
modified_files:
  - extensions/hook-common/src/install.rs
priority: low
ordinal: 53000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/hook-common/src/install.rs`

**What**: seven error sites in `install.rs` attach a fixed `&str` context and
drop the path the operation actually failed on:

- `:38` — `create_dir_all(&hooks_dir).context("failed to create .git/hooks directory")`
- `:160` — `read_to_string(hook_path).context("failed to read existing hook")`
- `:254` — `read_to_string(hook_path).context("failed to re-read existing hook before upgrade")`
- `:285` — `.context("failed to rename temp hook into place")` (names neither the stage nor the destination)
- `:347` — `.context("hook path has no parent directory")`
- `:351` — `.context("hook path has no filename")`
- `:391` — `set_permissions(...).context("failed to make hook executable")`

This is inconsistent with the rest of the same file and crate, which is
otherwise careful about it: `stage_hook_payload`, `write_hook_payload`,
`paths::canonical_git_dir`, `paths::canonical_subdir`, and
`config::ensure_config_command` all use `with_context(|| format!("... {}", x.display()))`,
and the `commands`-is-not-a-table context was widened for exactly this reason
in TASK-1895.

**Why it matters**: `install_hook` is reached from `ops <hook>-install`, and
the whole point of the crate's error posture is that the operator can tell
*which* file in *which* worktree refused. `git_dir` is a parameter, so the
`.git` in play is not necessarily the one under the operator's cwd — the same
argument TASK-1895 already made for `.ops.toml`. With a bare "failed to make
hook executable" on a machine with several worktrees, the operator has nothing
to act on but the message text.

Low severity: diagnostics only, no behaviour change.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Every std::fs error site in install.rs names the path it operated on, using with_context(|| format!(...display())) as the rest of the crate does
- [x] #2 The rename context names both the staged path and the destination hook path
- [x] #3 A test asserts that at least one install failure message contains the offending path
<!-- AC:END -->
