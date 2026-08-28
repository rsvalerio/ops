---
id: TASK-1752
title: >-
  TEST-23: prompt_hook_install_noninteractive_reports_failure leaks
  OPS_NONINTERACTIVE=1 into the rest of the test binary on assertion failure
status: To Do
assignee:
  - TASK-1982
created_date: '2026-08-27 11:15'
updated_date: '2026-08-28 14:08'
labels:
  - code-review-rust
  - test-quality
dependencies: []
modified_files:
  - crates/cli/src/subcommands.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/subcommands.rs:604-614`

**What**:

```rust
#[test]
#[serial_test::serial]
fn prompt_hook_install_noninteractive_reports_failure() {
    std::env::set_var("OPS_NONINTERACTIVE", "1");
    let cfg = Config::default();
    let code = prompt_hook_install(&cfg, &pre_hook_cmd::COMMIT_OPS)
        .expect("noninteractive bail must not error");
    std::env::remove_var("OPS_NONINTERACTIVE");
    assert_eq!(format!("{code:?}"), format!("{:?}", ExitCode::FAILURE));
}
```

The cleanup is a plain statement, not a `Drop` guard, and it sits *after* a fallible `.expect(...)`. If `prompt_hook_install` ever returns `Err` — the failure this test exists to catch — the `expect` panics, `remove_var` never runs, and `OPS_NONINTERACTIVE=1` stays set for the remaining lifetime of the test process. `serial_test::serial` serialises execution; it does not restore process state.

`env_flag_enabled("OPS_NONINTERACTIVE")` is read by `noninteractive_install_blocked` (`subcommands.rs:138`), which every hook-install path consults, so the leak silently forces the non-interactive branch for every later test in the binary. The result is one genuine failure followed by an unpredictable set of secondary failures or false passes, ordered by however the harness scheduled the remaining tests.

The same module already has the correct tool, defined 40 lines above and used by exactly one test: `EnvVarGuard` (`subcommands.rs:554-574`), an RAII guard that snapshots the original value and restores it on drop. It currently only offers `unset`; a `set(name, value)` constructor is the missing half.

`env_flag_enabled_treats_falsy_as_off` (line 445) has the same shape — a bare `remove_var` at line 466 after 18 fallible `assert!`s — though it leaks a test-only variable name and so is lower impact.

**Why it matters**: TEST-23 / TEST-18 — shared global state needs cleanup that survives a panic, and process environment is the most global state a test can touch. The leak converts a single real regression into a cascade of unrelated failures, which is precisely the signal-destroying outcome the rule guards against; and the fix is to use a guard that already exists in the same file.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 EnvVarGuard gains a set(name, value) constructor alongside unset, keeping the snapshot-and-restore-on-drop behaviour
- [ ] #2 prompt_hook_install_noninteractive_reports_failure sets OPS_NONINTERACTIVE through the guard so the variable is restored even when the test panics
- [ ] #3 env_flag_enabled_treats_falsy_as_off sets OPS_NONINTERACTIVE_TEST through the same guard instead of a trailing remove_var
- [ ] #4 No test in crates/cli/src calls std::env::set_var or remove_var outside an RAII guard
<!-- AC:END -->
