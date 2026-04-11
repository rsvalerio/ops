---
id: TASK-0021
title: "config/mod.rs has 601 lines spanning 4 distinct concerns"
status: Triage
assignee: []
created_date: '2026-04-09 19:25:00'
labels: [rust-code-quality, CQ, ARCH-1, ARCH-8, low, crate-core]
dependencies: []
---

## Description

**Location**: `crates/core/src/config/mod.rs:1-601`
**Anchor**: `mod config`
**Impact**: The file owns: (1) config struct definitions (`Config`, `ConfigOverlay`, `OutputConfig`, etc.), (2) merge logic (`merge_config`, `merge_field`, `merge_indexmap`), (3) I/O (`load_config`, `read_config_file`, `read_conf_d_files`, `load_global_config`), and (4) template generation (`init_template`, `default_ops_toml`, `InitSections`). The merge logic at lines 392-422 is inconsistent — `commands`/`themes`/`tools` use helpers (`merge_indexmap`/`merge_field`) while `data` and `extensions` use raw field mutation at a different abstraction level (READ-6).

**Notes**:
Natural split: `config/types.rs` (struct definitions), `config/merge.rs` (merge logic), `config/loader.rs` (file I/O). This improves testability — the loader can't currently be tested without touching the filesystem. Also make `data`/`extensions` merge consistent with the helper-based approach used by other fields. Low severity because the file is well-organized despite its size.
