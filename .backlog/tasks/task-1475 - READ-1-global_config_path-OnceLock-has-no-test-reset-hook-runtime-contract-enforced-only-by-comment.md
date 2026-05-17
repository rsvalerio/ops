---
id: TASK-1475
title: >-
  READ-1: global_config_path OnceLock has no test reset hook; runtime contract
  enforced only by comment
status: To Do
assignee:
  - TASK-1481
created_date: '2026-05-16 10:06'
updated_date: '2026-05-17 07:06'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/loader.rs:430-447`

**What**: `GLOBAL_CONFIG_PATH` is a `OnceLock<Option<PathBuf>>` resolved on first call with no test-support reset. The accompanying doc-comment warns "tests MUST set env before any code path triggers load_config" — that's a runtime contract enforced only by comment.

**Why it matters**: `resolve_global_config_path` is already `pub(crate)` precisely so tests can drive the matrix, but there's no way to clear the resolved cache between scenarios in a single binary. The `#[serial_test::serial]` discipline elsewhere (load_config_call_count) shows the team's pattern for guarding global state — global config path is missing it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Add a #[cfg(any(test, feature = "test-support"))] reset hook (e.g. pub fn reset_global_config_path_cache()) that takes a unique-token argument so accidental production calls don't compile
- [ ] #2 Audit tests that mutate XDG_CONFIG_HOME/APPDATA/HOME and ensure they call the reset hook before the first relevant load
<!-- AC:END -->
