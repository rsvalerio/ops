---
id: TASK-1419
title: >-
  PERF-3: global_config_path re-reads XDG_CONFIG_HOME/APPDATA/HOME on every
  load_config call
status: Done
assignee:
  - TASK-1455
created_date: '2026-05-13 18:17'
updated_date: '2026-05-13 22:59'
labels:
  - code-review-rust
  - perf
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/loader.rs:318`

**What**: `global_config_path` performs `std::env::var_os` lookups on `XDG_CONFIG_HOME`, `APPDATA`, and falls back through `crate::paths::home_dir()` (another env read) on every `load_config()` call. The env vars are process-stable in practice and `load_config` is the CLI startup critical path. Mirrors the existing OnceLock discipline used by `TMPDIR_DISPLAY` and `OPS_TOML_MAX_BYTES`.

**Why it matters**: Tiny but on the hot startup path. Cache the resolved Option<PathBuf> behind a `OnceLock` so env lookups happen at most once per process.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 memoize global_config_path result behind a OnceLock so env reads happen at most once
- [x] #2 preserve existing source-of-base-dir tracing::debug on first resolution only
- [x] #3 document process-lifetime contract alongside TMPDIR_DISPLAY equivalent
<!-- AC:END -->
