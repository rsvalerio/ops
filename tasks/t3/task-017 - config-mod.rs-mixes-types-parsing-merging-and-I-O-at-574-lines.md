---
id: TASK-017
title: 'config/mod.rs mixes types, parsing, merging, and I/O at 574 lines'
status: To Do
assignee: []
created_date: '2026-04-07 12:00:00'
updated_date: '2026-04-07 22:48'
labels:
  - rust-code-quality
  - CQ
  - ARCH-1
  - ARCH-3
  - medium
  - effort-M
  - crate-core
dependencies: []
ordinal: 17000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Location**: `crates/core/src/config/mod.rs:1-574`
**Anchor**: `mod config`
**Impact**: The file combines five distinct responsibilities in 574 lines: (1) data structures (Config, OutputConfig, CommandSpec, etc.), (2) TOML parsing via `config` crate, (3) merge logic (merge_config, merge_field, merge_indexmap), (4) file I/O (load_config, read_config_file, global_config_path), and (5) environment variable handling (merge_env_vars). This exceeds the 500-line ARCH-1 threshold and mixes unrelated concerns per ARCH-3.

**Notes**:
Suggested split:
- `config/types.rs` — Config, OutputConfig, CommandSpec, CompositeCommandSpec, ExecCommandSpec, DataConfig, InitSections, CommandId (data structures and serde derives)
- `config/merge.rs` — merge_config, merge_field, merge_indexmap, merge_env_vars (merge logic)
- `config/mod.rs` — load_config, read_config_file, global_config_path, load_global_config, merge_conf_d, init_template (orchestration and I/O, re-exports types)

This keeps the public API surface in mod.rs while separating concerns. The tests module can stay alongside the code it tests or move to a dedicated test file.
<!-- SECTION:DESCRIPTION:END -->
