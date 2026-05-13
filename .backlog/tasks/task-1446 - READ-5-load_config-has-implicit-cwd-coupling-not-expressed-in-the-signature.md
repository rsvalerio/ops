---
id: TASK-1446
title: 'READ-5: load_config has implicit cwd coupling not expressed in the signature'
status: To Do
assignee:
  - TASK-1453
created_date: '2026-05-13 18:44'
updated_date: '2026-05-13 19:09'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/loader.rs:166-194`

**What**: `load_config` uses `PathBuf::from(".ops.toml")` and `merge_conf_d(...)` (which hits `Path::new(".ops.d")`) implicitly relying on the process cwd, while `load_global_config` resolves a fully absolute path. Nothing in the function signature or doc comment says "this function is cwd-sensitive" — a future async/concurrent caller spawning across cwds will silently load a different `.ops.toml` than expected.

**Why it matters**: This is the cause of the `#[serial_test::serial]` discipline in the test module. The discipline is documented for tests but not for production callers. The function should either accept a `workspace_root: &Path` parameter (preferred) or rename/document its cwd-coupling explicitly.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 load_config accepts a workspace_root: &Path (or a thin load_config_at(workspace_root) wrapper exists) so the cwd coupling is in the type
- [ ] #2 Existing call sites are migrated to pass std::env::current_dir()? explicitly at the boundary
- [ ] #3 The doc comment for any remaining cwd-relative entry point states the invariant
<!-- AC:END -->
