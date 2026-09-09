---
id: TASK-2144
title: 'TEST-18: env-mutating tests are serialized only against each other while three sibling tests spawn subprocesses in parallel, racing setenv against environ reads'
status: To Do
assignee: []
created_date: '2026-09-08 07:02'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-rust
  - test-quality
dependencies: []
parent_task_id: 'TASK-2241'
modified_files:
  - extensions/run-before-push/src/lib.rs
priority: medium
ordinal: 59000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/run-before-push/src/lib.rs:422` (`skip_reason_reads_the_forwarded_env_var`), `extensions/run-before-push/src/lib.rs:447` (`should_skip_returns_false_by_default`)

**What**: Both env-mutating tests carry `#[serial_test::serial]`, which serializes them against *other serial tests only*. The same test binary contains three tests with no `serial` attribute that spawn child processes, and `std::process::Command::spawn` snapshots the process environment while building `envp`:

- `hook_script_is_valid_posix_sh` (:278)
- `hook_script_hands_ops_an_empty_stdin_and_forwards_refs_via_env` (:294)
- `hook_script_fails_closed_with_diagnostic_when_ops_is_missing` (:319)

Those run on the harness's other threads while `skip_reason_reads_the_forwarded_env_var` calls `EnvGuard::set(REF_UPDATES_ENV_VAR, ...)` / `EnvGuard::remove(...)` three times and `should_skip_returns_false_by_default` calls `EnvGuard::remove(SKIP_ENV_VAR)` — i.e. `setenv`/`unsetenv` on one thread concurrently with `environ` reads on another.

`ops_hook_common::test_helpers::EnvGuard`'s own doc states the contract and its limit: "Pair with `#[serial_test::serial]` to prevent races with other env-mutating tests: `std::env::set_var`/`remove_var` mutate process-wide state and race with concurrent `getenv` calls." The convention covers env-mutating siblings; it does not cover the three spawning ones, which never opt in.

The exposure is narrower here than in the sibling crate (three spawners, not eight, and no `CwdGuard` use), but the mechanism and the fix are the same shape. Filed separately from TASK-2119 because that task's scope is `extensions/run-before-commit/src/lib.rs` only.

**Why it matters**: A `setenv`/`getenv` data race is undefined behaviour, not merely flaky — glibc can reallocate `environ` under a concurrent reader. The symptom is a rare, unreproducible spawn failure or a child inheriting a torn environment, which reads as a fault in the fake-ops harness rather than as a race. The hazard also grows silently: nothing stops a newly added spawning test from landing without the attribute.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 No test in this binary can observe the process environment being mutated by another test: either every process-spawning test is serialized with the env mutators, or the mutating tests stop touching process-global state
- [ ] #2 The chosen mechanism is documented next to the guards so a newly added spawning test cannot silently opt out of it
- [ ] #3 cargo test -p ops-run-before-push passes under the default parallel harness
<!-- AC:END -->
