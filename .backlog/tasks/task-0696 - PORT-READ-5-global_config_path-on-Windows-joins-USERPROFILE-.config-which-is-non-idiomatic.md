---
id: TASK-0696
title: >-
  PORT/READ-5: global_config_path on Windows joins USERPROFILE/.config which is
  non-idiomatic
status: To Do
assignee:
  - TASK-0743
created_date: '2026-04-30 05:26'
updated_date: '2026-04-30 06:07'
labels:
  - code-review-rust
  - READ
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/loader.rs:185-193`

**What**: `global_config_path` falls back to `$HOME` then `$USERPROFILE` when `XDG_CONFIG_HOME` is unset, then unconditionally appends `.config/ops/config`. On Windows the conventional location is `%APPDATA%\\ops\\config.toml` (or `dirs::config_dir`); putting it under `~/.config/` works only by accident on shells where USERPROFILE happens to back the call. Combined with the surrounding `with_extension("toml")` retry, the loader looks at `C:\\Users\\X\\.config\\ops\\config.toml` instead of the platform-standard path, so a Windows user editing the obvious location sees no effect.

**Why it matters**: The codebase advertises Windows support (test_utils platform_exec_spec, sleep_cmd Windows branch). Silent path divergence between docs/expectation and behavior produces "config not loading on Windows" reports with no log signal — `read_config_file` returns Ok(None) on NotFound.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Use dirs::config_dir() (or APPDATA-aware lookup) on Windows
- [ ] #2 Document the resolved path in tracing::debug for diagnosis
- [ ] #3 Add a Windows-targeted unit test that verifies the chosen base directory
<!-- AC:END -->
