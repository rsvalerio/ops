---
id: TASK-1322
title: >-
  PATTERN-1: prompt_hook_install dispatches install via stringly-typed match on
  hook_name despite HookOps.install_fn already encoding the install entry point
status: Done
assignee:
  - TASK-1382
created_date: '2026-05-11 20:55'
updated_date: '2026-05-12 22:59'
labels:
  - code-review-rust
  - pattern
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/subcommands.rs:118-154` (specifically lines 146-150)

**What**: `prompt_hook_install` accepts `hook_name: &str` and then dispatches to the install entry with:

```rust
match hook_name {
    "run-before-commit" => pre_hook_cmd::run_before_commit_install(config)?,
    "run-before-push" => pre_hook_cmd::run_before_push_install(config)?,
    other => anyhow::bail!("unknown hook: {other}"),
}
```

The `HookOps` descriptor table (lifted under TASK-0757 / DUP-1 TASK-1282) already carries an `install_fn: fn(&Config) -> anyhow::Result<()>` field for exactly this purpose. The caller `run_hook_dispatch` has `hook: &HookOps` in scope and passes only `hook.hook_name` down to `prompt_hook_install`, so the function then has to recover by string-matching a name it should never have had to compare on.

**Why it matters**:
1. The whole point of the `HookOps` constants (COMMIT_OPS, PUSH_OPS) is that adding a new hook is a one-table edit. The stringly-typed dispatch defeats that: adding `run-before-merge` means editing both the `pre_hook_cmd` const table *and* this match arm, with no compile-time signal that the second edit is missing — only a runtime `anyhow::bail!("unknown hook: run-before-merge")` discovered by users.
2. The `other => bail!` arm is unreachable in practice (every call site passes one of the two `HookOps::hook_name` values), so it's a dead branch with no test coverage that exists solely because the function is typed too loosely.
3. The architectural drift is the same one TASK-0757 and TASK-1282 collapsed elsewhere in this module.

Fix: thread `&HookOps` to `prompt_hook_install` instead of `&str`. The function already builds the prompt label and note text from `hook_name`, which is reachable from `hook.hook_name`. The match arm collapses to `if answer { (hook.install_fn)(&config)?; ... }`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Change prompt_hook_install signature to accept &HookOps instead of (Config, &str)
- [ ] #2 Replace the run-before-commit/run-before-push match with a single call through hook.install_fn
- [ ] #3 Remove the unreachable 'unknown hook: {other}' bail arm
- [ ] #4 Existing tests still pass; new test asserts that adding a HookOps to the constants table makes the dispatch reachable without further edits to subcommands.rs
<!-- AC:END -->
