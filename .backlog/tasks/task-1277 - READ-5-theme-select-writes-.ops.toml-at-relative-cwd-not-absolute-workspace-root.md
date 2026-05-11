---
id: TASK-1277
title: >-
  READ-5: theme select writes .ops.toml at relative cwd, not absolute workspace
  root
status: To Do
assignee:
  - TASK-1303
created_date: '2026-05-11 15:25'
updated_date: '2026-05-11 16:48'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/theme_cmd.rs:189`

**What**: `update_theme_in_config` does `PathBuf::from(".ops.toml")` rather than anchoring to the workspace root threaded from `run()`. `about_cmd.rs` was explicitly fixed under TASK-0578 to receive `workspace_root: &Path` and write next to the loaded config; theme select was missed.

**Why it matters**: Running `ops theme select` from a subdirectory writes a new `.ops.toml` containing only `[output] theme = "..."` in the subdir, while the workspace's real config (which was read to populate the prompt) is left untouched. Operators experience a select that visibly succeeds but appears to have no effect.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 run_theme_select takes workspace_root: &Path threaded from run() (matching run_about_setup signature shape)
- [ ] #2 update_theme_in_config(workspace_root, theme_name) joins to an absolute path before calling edit_ops_toml
- [ ] #3 Regression test mirroring save_about_fields_writes_to_workspace_root_from_subdir
- [ ] #4 Comment links READ-5 / TASK-0578 so the symmetry with about/init is documented
<!-- AC:END -->
