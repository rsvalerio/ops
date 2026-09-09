---
id: TASK-2119
title: 'TEST-18: env- and cwd-mutating hook tests are serialized only against each other, while eight parallel tests in the same binary spawn subprocesses'
status: To Do
assignee: []
created_date: '2026-09-08 06:54'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-rust
  - test-quality
dependencies: []
parent_task_id: 'TASK-2241'
modified_files:
  - extensions/run-before-commit/src/lib.rs
priority: medium
ordinal: 36000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/run-before-commit/src/lib.rs:684` (`has_staged_files_applies_the_env_timeout`), `extensions/run-before-commit/src/lib.rs:653`, `extensions/run-before-commit/src/lib.rs:713`

**What**: The env-mutating tests carry `#[serial_test::serial]`, which serializes them against *other serial tests only*. The same test binary also contains tests with no `serial` attribute that spawn child processes, and `std::process::Command::spawn` snapshots the process environment (`environ`) while building `envp`:

- `hook_script_is_valid_posix_sh` (:192)
- `hook_script_reports_a_missing_ops_binary_by_name` (:208)
- `hook_script_honours_the_bypass_when_ops_is_missing` (:240)
- `has_staged_files_lossily_decodes_invalid_utf8_stderr` (:489)
- `has_staged_files_errors_when_git_binary_missing` (:505)
- `has_staged_files_times_out_on_hanging_git` (:518)
- `has_staged_files_captures_late_stderr_within_drain_grace` (:550)
- `has_staged_files_handles_large_output_without_deadlock` (:595)

Those run on the harness's other threads at the same time as `has_staged_files_applies_the_env_timeout` calls `EnvGuard::set("PATH", ...)` (:691) and `EnvGuard::set(TIMEOUT_ENV_VAR, "1")` (:692) — i.e. `setenv` on one thread concurrently with `getenv`/`environ` reads on another. That is the exact race `ops_core::test_utils::EnvGuard`'s own doc warns about ("`std::env::set_var`/`remove_var` mutate process-wide state and race with concurrent `getenv` calls"); the `serial` convention it prescribes does not cover non-serial siblings. `CwdGuard` (:657, :693, :716) takes its own process-wide mutex, but that mutex is likewise invisible to the eight tests above, which spawn while the process cwd is pointed at a tempdir.

**Why it matters**: A `setenv`/`getenv` data race is undefined behaviour, not merely flaky — glibc can reallocate `environ` under a concurrent reader. The observable symptom is a rare, unreproducible spawn failure or a child that inherits a torn environment, attributed to the fake-git harness rather than to the race. The crate already documents an unrelated ETXTBSY retry loop, so this binary is already known to be racy under parallelism.

<!-- scan confidence: candidates to inspect -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 No test in this binary can observe the process environment or cwd being mutated by another test: either every process-spawning test is serialized with the env/cwd mutators, or the mutating tests no longer touch process-global state (pass the program path and timeout explicitly)
- [ ] #2 The chosen mechanism is documented next to the guards so a newly added test cannot silently opt out of it
- [ ] #3 cargo test -p ops-run-before-commit passes with the default parallel harness
<!-- AC:END -->
