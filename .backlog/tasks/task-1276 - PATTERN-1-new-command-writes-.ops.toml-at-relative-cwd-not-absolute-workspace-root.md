---
id: TASK-1276
title: >-
  PATTERN-1: new-command writes .ops.toml at relative cwd, not absolute
  workspace root
status: To Do
assignee:
  - TASK-1303
created_date: '2026-05-11 15:25'
updated_date: '2026-05-11 16:48'
labels:
  - code-review-rust
  - pattern
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/new_command_cmd.rs:102`

**What**: `append_command_to_config` does `PathBuf::from(".ops.toml")` and hands the relative path to `edit_ops_toml`. `init_cmd.rs` (TASK-1066) and `about_cmd.rs` (TASK-0578) were both explicitly hardened to (a) capture cwd once and join to an absolute path to close the TOCTOU window between create and parent fsync, and (b) anchor the write to the workspace root threaded from `run()`, not the user's cwd. new-command was missed by both fixes.

**Why it matters**: Running `ops new-command` from a subdirectory of a workspace writes a fresh `.ops.toml` in the subdir instead of updating the one the rest of the CLI loaded, silently splitting config state. A cwd change mid-edit (signal handler, threaded inquire) can also land the create and parent fsync in different directories.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 run_new_command takes a workspace_root: &Path (mirroring run_about_setup) and threads it from the caller in run()
- [ ] #2 append_command_to_config joins workspace_root with ".ops.toml" once to get an absolute path
- [ ] #3 Regression test: from a subdir, the command updates workspace_root/.ops.toml and does not create a stray subdir/.ops.toml
- [ ] #4 Cross-reference TASK-0578 / TASK-1066 in the comment so the next sweep doesn't reintroduce it
<!-- AC:END -->
