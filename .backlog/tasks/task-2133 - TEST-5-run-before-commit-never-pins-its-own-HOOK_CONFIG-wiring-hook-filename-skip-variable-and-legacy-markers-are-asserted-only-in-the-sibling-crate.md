---
id: TASK-2133
title: 'TEST-5: run-before-commit never pins its own HOOK_CONFIG wiring - hook filename, skip variable and legacy markers are asserted only in the sibling crate'
status: To Do
assignee: []
created_date: '2026-09-08 06:55'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-rust
  - test-quality
dependencies: []
parent_task_id: 'TASK-2241'
modified_files:
  - extensions/run-before-commit/src/lib.rs
priority: low
ordinal: 49000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/run-before-commit/src/lib.rs:76` (`impl_hook_wrappers!` invocation), `extensions/run-before-commit/src/lib.rs:124` (tests)

**What**: The macro invocation is this crate's entire behavioural contribution beyond `HOOK_SCRIPT` and `has_staged_files`: it names the hook file `"pre-commit"`, binds `SKIP_ENV_VAR`, lists three `legacy_markers` and sets `command_help`. No test in this crate asserts any of it.

- `install_hook_updates_legacy_before_commit_hook` (:282) and `install_hook_updates_legacy_pre_commit_hook` (:304) pre-create `hooks/pre-commit`, then assert only on the *content* of the path `install_hook` returns — so a typo changing `hook_filename` to `"pre-push"` would write a new file, return that path, and both tests would still pass while the pre-created hook the test set up is left untouched.
- the third legacy marker, `"ops run-before-commit"`, has no upgrade test at all.
- `command_help` and `skip_env_var` are asserted nowhere.

The sibling crate does exactly this pinning (`extensions/run-before-push/src/lib.rs:497-532`, "HOOK_CONFIG is the only thing distinguishing this crate"), so the gap is asymmetric rather than a deliberate project position.

**Why it matters**: A wrong `hook_filename` installs a hook git never runs — the checks silently stop gating commits and every test stays green. The macro arguments are also precisely the values a copy-paste of this crate into a new hook would get wrong.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A test asserts HOOK_CONFIG.hook_filename == "pre-commit", HOOK_CONFIG.skip_env_var == SKIP_ENV_VAR, HOOK_CONFIG.name == NAME and HOOK_CONFIG.hook_script == HOOK_SCRIPT
- [ ] #2 A test asserts command_help is non-empty and names the hook, mirroring the run-before-push coverage
- [ ] #3 Each legacy marker, including "ops run-before-commit", is covered by an install_hook upgrade test
- [ ] #4 The install_hook tests assert the returned path's file name is pre-commit, so a wrong hook_filename fails them
<!-- AC:END -->
