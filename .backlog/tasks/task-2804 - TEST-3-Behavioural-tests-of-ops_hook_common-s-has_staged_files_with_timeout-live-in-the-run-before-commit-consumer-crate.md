---
id: TASK-2804
title: 'TEST-3: Behavioural tests of ops_hook_common''s has_staged_files_with_timeout live in the run-before-commit consumer crate'
status: To Do
assignee: []
created_date: '2026-11-18 18:46'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - TEST
dependencies: []
parent_task_id: 'TASK-2861'
modified_files:
  - extensions/run-before-commit/src/lib.rs
  - extensions/hook-common/src/git_state.rs
priority: medium
ordinal: 1000
dedup_key: 'TEST-3:extensions/run-before-commit/src/lib.rs:tests::has_staged_files_with'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/run-before-commit/src/lib.rs:458-683` (tests `has_staged_files_false_when_index_empty`, `..._true_when_file_staged`, `..._true_when_only_a_deletion_is_staged`, `..._true_when_only_a_type_change_is_staged`, `..._errors_outside_git_repo`, `..._lossily_decodes_invalid_utf8_stderr`, `..._errors_when_git_binary_missing`, `..._times_out_on_hanging_git`, `..._captures_late_stderr_within_drain_grace`, `..._handles_large_output_without_deadlock`, and helpers `has_staged_files_with`, `write_fake_git`, `retry_while_text_file_busy`)

**What**: These tests exercise only `ops_hook_common::git_state::has_staged_files_with_timeout` through its public API with an explicit program/dir — the module's own comment at `:155-162` says they are "not coverage of the production preflight". Yet `extensions/hook-common/src/git_state.rs` has no tests for `has_staged_files_with_timeout` at all; the shared probe's bounded wait, stderr drain, deletion/type-change semantics and ETXTBSY handling are covered only from a downstream crate.

**Why it matters**: The coverage sits away from the code it protects. A change to `git_state.rs` is not caught by running `ops-hook-common`'s tests; any other consumer (run-before-push) inherits the guarantees only by accident; and removing or refactoring this crate would silently drop the shared probe's whole behavioural suite. Per TEST-3 tests are placed by surface under test: these belong with `ops-hook-common` (its `tests/` dir or `git_state` test module), leaving here only the tests that pin what `has_staged_files` itself adds (cwd, env timeout, `"git"`, anyhow conversion — `:715-788`).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The has_staged_files_with_timeout behavioural tests (and their fake-git/ETXTBSY helpers) live in ops-hook-common
- [ ] #2 run-before-commit keeps only tests for what has_staged_files contributes on top of the shared probe
- [ ] #3 No net loss of scenarios: deletion, type change, non-repo, invalid UTF-8 stderr, missing binary, timeout, late stderr and large output are each still covered
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.28 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
