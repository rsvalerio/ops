---
id: TASK-0011
title: Config loading pipeline in loader.rs has no unit tests
status: Done
assignee: []
created_date: '2026-04-10 18:00:00'
updated_date: '2026-04-11 09:55'
labels:
  - rust-test-quality
  - TQ
  - TEST-5
  - TEST-6
  - medium
  - crate-core
dependencies: []
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Location**: `crates/core/src/config/loader.rs:1-143`
**Anchor**: `fn load_config`, `fn merge_env_vars`, `fn read_conf_d_files`, `fn load_global_config`
**Impact**: The primary config entry point `load_config()` and its internal helpers have no unit tests. This function orchestrates default config parsing, global config merge, local `.ops.toml` merge, `.ops.d/` directory merge, and environment variable override — five distinct merge steps with multiple error branches. Currently only exercised through CLI-level integration tests, which don't isolate individual loading stages.

**Notes**:
- `read_conf_d_files()` (line 83): sorting guarantee, `.toml` extension filtering, and `NotFound` vs other IO error handling are all untested at unit level
- `merge_env_vars()` (line 18): `OPS__` prefix detection guard and the env-to-overlay deserialization path are untested — requires env var injection in tests
- `load_global_config()` (line 131): `XDG_CONFIG_HOME` vs `HOME` fallback, `.toml` extension probing, and early-return-on-first-match are untested
- `read_config_file()` and `global_config_path()` ARE tested in `config/tests.rs` — the gap is in the orchestration functions that call them
- Recommend adding tests with `tempfile::tempdir()` for filesystem paths, `std::env::set_var` (under mutex) for env vars, covering: (1) `read_conf_d_files` sorts and filters correctly, (2) `merge_env_vars` applies OPS__-prefixed overrides, (3) `load_global_config` prefers `.toml` extension
<!-- SECTION:DESCRIPTION:END -->
