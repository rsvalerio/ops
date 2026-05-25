---
id: TASK-1421
title: >-
  ERR-1: load_config does not record which source's parse failure caused the
  abort
status: Done
assignee:
  - TASK-1453
created_date: '2026-05-13 18:18'
updated_date: '2026-05-13 20:43'
labels:
  - code-review-rust
  - err
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/loader.rs:167`

**What**: `load_config` calls `load_global_config`, `read_config_file(.ops.toml)`, `merge_conf_d`, and `merge_env_vars` in sequence with bare `?` propagation. When `read_config_file(local_path)` fails, the error includes `.ops.toml` via `read_capped_toml_file`'s context, but `merge_env_vars` returns its anyhow chain without a top-level "while loading config from env" wrapper, so a deserialization failure on `OPS__OUTPUT__THEME` renders without indicating the source layer.

**Why it matters**: Operators reading "failed to deserialize OPS__ env config" already see the keys; the local path also includes path context. The gap is the order/layer breadcrumb — a future refactor of layer order would be invisible in error messages. Add `with_context` at each call so the chain reads `"loading global config" -> ... -> root cause`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 wrap each load step (load_global_config, local read, merge_conf_d, merge_env_vars) with with_context naming the source layer
- [ ] #2 rendered error chain includes layer-name at top level for every failure path
- [ ] #3 regression test injects a parse failure into one layer and pins the rendered chain prefix
<!-- AC:END -->
