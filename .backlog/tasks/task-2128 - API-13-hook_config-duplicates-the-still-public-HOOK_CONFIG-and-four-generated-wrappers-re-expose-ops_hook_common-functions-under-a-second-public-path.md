---
id: TASK-2128
title: 'API-13: hook_config() duplicates the still-public HOOK_CONFIG, and four generated wrappers re-expose ops_hook_common functions under a second public path'
status: To Do
assignee: []
created_date: '2026-09-08 06:55'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-rust
  - api-design
dependencies: []
parent_task_id: 'TASK-2247'
modified_files:
  - extensions/hook-common/src/lib.rs
  - extensions/run-before-commit/src/lib.rs
  - extensions/run-before-push/src/lib.rs
priority: low
ordinal: 44000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/hook-common/src/lib.rs:118` (`impl_hook_wrappers!`), as instantiated at `extensions/run-before-commit/src/lib.rs:76`

**What**: The macro emits both `pub const HOOK_CONFIG` and `pub fn hook_config() -> HookConfig` returning it (lines 118-129). Two public paths to the same value, with nothing in the doc saying which one a caller should use, and neither is referenced anywhere in `ops-run-before-commit` — `crates/cli/src/pre_hook_cmd.rs:10-16` reaches only for `find_git_dir`, `install_hook`, `ensure_config_command`, `SKIP_ENV_VAR`, `should_skip` and `has_staged_files`. In the sibling crate `hook_config()` exists solely so a test can assert it agrees with `HOOK_CONFIG` (`extensions/run-before-push/src/lib.rs:544-550`), which is a test of the duplication rather than of behaviour.

The other four wrappers (`should_skip`, `find_git_dir`, `install_hook`, `ensure_config_command`) are one-line forwards to the identically named `ops_hook_common` functions with `&HOOK_CONFIG` prefilled. They are a defensible currying convenience, but the crate offers no signal that they are the only supported path — `ops_hook_common::find_git_dir` takes no config at all and is equally public, so the same function is reachable under two names with the same signature.

**Why it matters**: The rule of one public path per item exists so that "which of these do I call" is never a question and so that a change has one place to land. Here it costs a per-crate test that only checks a copy against its original, and it leaves `HOOK_CONFIG`/`hook_config()` as dead API in this crate.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 There is one public path to the hook config: either the const or the accessor, not both
- [ ] #2 The run-before-push test that asserts hook_config() equals HOOK_CONFIG is deleted along with the duplicate path, not left asserting an identity
- [ ] #3 Every generated wrapper that survives states in its doc that it is the config-bound form of the ops_hook_common function of the same name
- [ ] #4 cargo build and cargo test pass for the workspace; crates/cli/src/pre_hook_cmd.rs still resolves every symbol it names
<!-- AC:END -->
