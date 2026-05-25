---
id: TASK-1312
title: >-
  TEST-25: handlers_do_not_reload_config asserts run_hook_dispatch path it never
  exercises
status: Done
assignee:
  - TASK-1386
created_date: '2026-05-11 20:22'
updated_date: '2026-05-12 23:42'
labels:
  - code-review-rust
  - test-quality
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/subcommands.rs:309-358`

**What**: `handlers_do_not_reload_config` claims to pin two contracts: (a) `cli_data_context` must not reload `.ops.toml`, and (b) `run_hook_dispatch` must not reload it either (assertion at line 353-357 says "run_hook_dispatch must not reload .ops.toml"). The test fixture's config (lines 312-318) only defines `echo_test`, not `run-before-commit`, so the dispatch call at line 347-351 hits `if !config.commands.contains_key(hook.hook_name) { return prompt_hook_install(...) }` and short-circuits into `prompt_hook_install`. With `OPS_NONINTERACTIVE=1` set on line 346, `prompt_hook_install` returns `Ok(ExitCode::FAILURE)` before reaching the install confirmation, the hook command resolution, or `run_external_command`/`build_runner` — the actual dispatch branch where a config-reload regression would live.

The assertion that follows therefore passes trivially: none of the executed code calls `load_config*` anyway. The test name and message describe a stronger guarantee than the executed branch enforces.

**Why it matters**: Per TEST-25 the test name and assertion message claim path-specific behaviour the test does not exercise. A regression that re-loads `.ops.toml` inside `run_hook_dispatch`'s configured-command branch (i.e. when the hook command is actually defined and we route through `run_external_command`) would not be caught by this test, despite TASK-0427's stated invariant ("a single CLI invocation reads .ops.toml exactly once") being the reason the test exists.

<!-- scan confidence: verified by reading subcommands.rs:159-195 (run_hook_dispatch) and subcommands.rs:118-154 (prompt_hook_install) — the early-return on missing hook command and the OPS_NONINTERACTIVE bail are unambiguous -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Test exercises run_hook_dispatch on the configured-command branch (config contains a run-before-commit command spec) so the dispatch → run_external_command path is the one whose load_config_call_count is pinned
- [x] #2 OR the test is split into two named tests so each assertion (cli_data_context, run_hook_dispatch) is paired with a branch that actually invokes the code under that name
- [x] #3 load_config_call_count assertion message accurately reflects the branch executed
<!-- AC:END -->
