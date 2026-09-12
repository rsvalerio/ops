---
id: TASK-2076
title: 'TEST-18: results.rs env-mutation test races the process-global OUTPUT_BYTE_CAP OnceLock without serial'
status: Done
assignee: []
created_date: '2026-09-07 22:57'
updated_date: '2026-09-09 18:34'
labels:
  - code-review-rust
  - tests
dependencies: []
parent_task_id: 'TASK-2241'
modified_files:
  - crates/runner/src/command/results.rs
priority: medium
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/results.rs:427-445`

**What**: `output_byte_cap_is_memoized_across_calls` mutates the process-global `OPS_OUTPUT_BYTE_CAP` env var with no `#[serial_test::serial]` guard, unlike the sibling env-knob tests in `command/parallel.rs` (`resolve_max_parallel_*`) and `exec.rs` (`drain_grace_knob_tests`), which all carry serial annotations. The `SAFETY:` comment justifying the bare `set_var` — "tests under `cargo test` run on a single thread per binary" — is factually wrong: the default libtest harness runs tests on multiple threads; only the project's nextest gate isolates per process.

**Why it matters**: TEST-18 — unserialised process-env mutation is a cross-test race. The `OUTPUT_BYTE_CAP` `OnceLock` is read by every `from_streamed`/`spawn_capped` test in the same binary; if this test's `set_var(.., "1")` lands before another test's first `output_byte_cap()` call initializes the OnceLock, that test sees `cap = 1` and `command_output_from_streamed_success`-style assertions (`stdout == "hello world"`, 11 bytes > cap 1 → truncated) fail intermittently under plain `cargo test`. The race is dormant only because the CI gate uses nextest; any dev or tool running `cargo test` directly hits it. The incorrect SAFETY rationale also invites copy-paste of the same unsound claim into future env tests.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Test carries #[serial_test::serial(env_output_cap)] (a fresh serial key or the existing env group), matching the parallel.rs / exec.rs env-knob tests
- [x] #2 The SAFETY comment states the real isolation mechanism (serialisation and/or nextest per-process isolation), not the false single-thread-per-binary claim
- [x] #3 Plain

<!-- AC:END -->
