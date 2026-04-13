---
id: TASK-0006
title: 'config/mod.rs is a god module mixing types, loading, merging, and init'
status: Done
assignee: []
created_date: '2026-04-10 16:00:00'
updated_date: '2026-04-11 09:55'
labels:
  - rust-code-quality
  - CQ
  - ARCH-1
  - medium
  - crate-core
dependencies: []
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Location**: `crates/core/src/config/mod.rs:1-574`
**Anchor**: `mod.rs` (entire module)
**Impact**: The config module houses 48+ symbols — 10+ structs/enums, serde default helpers, config loading functions, merge logic, init template generation, and environment variable handling. These are distinct concerns that make the file hard to navigate and force unrelated changes to touch the same module.

**Notes**:
The refactoring is already in progress: `loader.rs` (143 lines) and `merge.rs` (66 lines) exist as untracked files in git with the loading and merge logic extracted. However, `mod.rs` still contains the original code alongside type definitions — the old functions haven't been removed and the new submodules aren't wired up yet.

Suggested split (completing the in-progress refactor):
- `mod.rs` — thin entry point: module declarations, re-exports, central types (`Config`, `OutputConfig`, `CommandSpec`, `ExecCommandSpec`, `CompositeCommandSpec`), serde defaults
- `loader.rs` — config loading (`load_config`, `read_config_file`, `read_conf_d_files`, `load_global_config`, `global_config_path`, `merge_env_vars`) — **already exists**
- `merge.rs` — overlay merge logic (`merge_config`, `merge_field`, `merge_indexmap`, `merge_output`) — **already exists**
- `init.rs` or inline — `InitSections`, `init_template`, `default_ops_toml` (init is a distinct concern from loading/merging)

ARCH-1: module >500 lines with mixed unrelated concerns. ARCH-8: mod.rs should be a thin entry point when submodules exist.
<!-- SECTION:DESCRIPTION:END -->
