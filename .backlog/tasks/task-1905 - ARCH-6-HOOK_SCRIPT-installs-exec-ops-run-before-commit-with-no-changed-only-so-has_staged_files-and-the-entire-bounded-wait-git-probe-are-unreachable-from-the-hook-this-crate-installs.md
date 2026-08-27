---
id: TASK-1905
title: >-
  ARCH-6: HOOK_SCRIPT installs 'exec ops run-before-commit' with no
  --changed-only, so has_staged_files and the entire bounded-wait git probe are
  unreachable from the hook this crate installs
status: Triage
assignee: []
created_date: '2026-08-27 15:39'
labels:
  - code-review-rust
  - architecture
dependencies: []
modified_files:
  - extensions/run-before-commit/src/lib.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/run-before-commit/src/lib.rs:44` (`HOOK_SCRIPT`), `extensions/run-before-commit/src/lib.rs:62-96` (timeout constants + `has_staged_files`)

**What**: The script this crate writes into `.git/hooks/pre-commit` is:

```rust
const HOOK_SCRIPT: &str = "#!/usr/bin/env bash\nexec ops run-before-commit\n";
```

No `--changed-only`. The preflight is gated on exactly that flag (`crates/cli/src/subcommands.rs:221-233`):

```rust
if changed_only {
    match hook.preflight { Some((predicate, skip_msg)) => { if !predicate()? { ... return Ok(SUCCESS); } } ... }
}
```

`grep -rn "changed.only" --include=*.rs --include=*.toml .` over the workspace finds no producer of `changed_only = true` outside `args.rs` parsing and its own unit tests. So on the shipped hook path `has_staged_files` is never called, and with it the whole apparatus this crate exists to parameterise is dormant:

- `DEFAULT_GIT_TIMEOUT` / `TIMEOUT_ENV_VAR` / `MAX_GIT_TIMEOUT_SECS` (lines 65-71)
- `git_timeout_from_env` and its clamp WARN
- the bounded-wait probe, stderr drain thread, drain grace, and typed `HasStagedFilesError` in `ops_hook_common::git_state`

That machinery is the accumulated output of five closed tasks — TASK-0589 (bounded wait), TASK-0650 (pipe-buffer deadlock), TASK-0725 (busy-poll removal), TASK-0864 (drain grace), TASK-1150 (detached drain thread) — all justified in their descriptions by "pre-commit hooks run on the developer's critical path". None of it runs on a developer's commit today.

Two readings, and the crate does not say which is intended:

1. The hook *should* pass `--changed-only` (the README describes it as "skips when nothing is staged", i.e. the hook behaviour), and `HOOK_SCRIPT` is missing it — in which case every commit currently pays the full check suite even for an empty index, and the fix is one flag.
2. The preflight is deliberately opt-in for humans running the command by hand — in which case the doc comments on lines 62-71 that justify the timeout policy by "pre-commit hooks run on the developer's critical path" are describing a path that does not exist, and the cost of the machinery should be re-examined.

**Why it matters**: ARCH-6 (match abstraction to problem complexity; YAGNI) plus a live behavioural question. Either the shipped hook is missing its documented skip behaviour, or a subprocess-orchestration layer with a spawned drain thread, a channel, an env-tunable clamp and a four-variant error enum is carried, tested and maintained for a code path no installed hook reaches. Both readings need a decision; neither is recorded anywhere in the crate.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A decision is recorded in the crate: either HOOK_SCRIPT passes --changed-only, or a comment on HOOK_SCRIPT states that the preflight is intentionally manual-only
- [ ] #2 If the flag is added, a test asserts HOOK_SCRIPT contains --changed-only and the README statement about skipping when nothing is staged matches the installed hook
- [ ] #3 If the preflight stays manual-only, the doc comments on DEFAULT_GIT_TIMEOUT and MAX_GIT_TIMEOUT_SECS stop justifying themselves by 'pre-commit hooks run on the developer's critical path'
<!-- AC:END -->
