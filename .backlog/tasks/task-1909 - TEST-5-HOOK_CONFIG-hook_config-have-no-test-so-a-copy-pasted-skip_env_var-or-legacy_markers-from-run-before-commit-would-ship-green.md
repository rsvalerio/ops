---
id: TASK-1909
title: >-
  TEST-5: HOOK_CONFIG / hook_config() have no test, so a copy-pasted
  skip_env_var or legacy_markers from run-before-commit would ship green
status: Triage
assignee: []
created_date: '2026-08-27 15:39'
labels:
  - code-review-rust
  - test-quality
dependencies: []
modified_files:
  - extensions/run-before-push/src/lib.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/run-before-push/src/lib.rs:40-47` (`impl_hook_wrappers!` invocation), tests at `extensions/run-before-push/src/lib.rs:49-118`

**What**: `impl_hook_wrappers!` generates six public items in this crate — `HOOK_CONFIG`, `hook_config()`, `should_skip()`, `find_git_dir()`, `install_hook()`, `ensure_config_command()`. The macro's whole job is to bind this crate's constants into `HOOK_CONFIG`, and that binding is the only thing that distinguishes `ops-run-before-push` from `ops-run-before-commit` (the two crates are otherwise byte-identical in shape). Nothing tests it:

- `HOOK_CONFIG` and `hook_config()` are never referenced by any test in the workspace (`grep -rn 'HOOK_CONFIG\|hook_config' --include=*.rs`).
- `extension_constants` (line 105) deliberately pins `NAME`, `SHORTNAME`, `DESCRIPTION` and `HOOK_SCRIPT` against external sources of truth, then stops before `HOOK_CONFIG`.
- `should_skip_returns_false_by_default` (line 70) reads `EnvGuard::remove(SKIP_ENV_VAR)` and asserts `!should_skip()`. Both sides go through the same const, so it passes regardless of which env var the macro was actually handed.
- `install_hook_updates_legacy_before_push_hook` (line 78) is the one test that would catch a wrong `hook_filename`, and only incidentally.

So of the six macro arguments, only `name`, `hook_script` and (accidentally) `hook_filename` are pinned. `skip_env_var`, `legacy_markers` and `command_help` are unpinned, and they are exactly the three a copy-paste from the sibling crate gets wrong.

Concretely, if line 44 read `skip_env_var: ops_run_before_commit::SKIP_ENV_VAR` — or if `SKIP_ENV_VAR` at line 38 were typo'd — the entire suite stays green while `SKIP_OPS_RUN_BEFORE_PUSH` becomes inert (the operator's documented escape hatch on a gate that blocks pushes silently stops working) and `SKIP_OPS_RUN_BEFORE_COMMIT` starts disabling the push hook as a side effect. Likewise a `legacy_markers` list carrying the commit crate's markers would make `install` refuse to upgrade a real legacy pre-push hook, or worse, claim an unrelated one.

**Why it matters**: TEST-5 — public API items with no test at all, where the untested item is the crate's entire reason to exist. The fix is cheap because `HOOK_CONFIG` is already `pub`: a handful of `assert_eq!` lines in `extension_constants` pin every field against the crate constants and the literal `"pre-push"`. This is distinct from TASK-1884, which covers `ops_hook_common::should_skip`'s value-parsing matrix in `extensions/hook-common/src/lib.rs`; that task tests what `should_skip` does with a value, this one tests that this crate hands it the right variable name at all. Both gaps have to close for the escape hatch to be covered end to end.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A test asserts HOOK_CONFIG.hook_filename == "pre-push" against the literal, not against another const
- [ ] #2 A test asserts HOOK_CONFIG.skip_env_var == SKIP_ENV_VAR and that SKIP_ENV_VAR == "SKIP_OPS_RUN_BEFORE_PUSH" against the literal name
- [ ] #3 A test asserts HOOK_CONFIG.name == NAME, HOOK_CONFIG.hook_script == HOOK_SCRIPT, and that command_help is non-empty and mentions push rather than commit
- [ ] #4 A test asserts every entry of HOOK_CONFIG.legacy_markers refers to a push hook (no 'commit' marker leaked in) and that the current HOOK_SCRIPT's command line is covered by one of them
- [ ] #5 hook_config() is exercised by at least one test so the generated accessor is not dead public surface
<!-- AC:END -->
