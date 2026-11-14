---
id: TASK-2213
title: 'TEST-18: serial_test guards the env-mutating cap tests against each other but not against the parallel tests that read the same environment'
status: Done
assignee: []
created_date: '2026-09-08 07:21'
updated_date: '2026-09-09 18:58'
labels:
  - code-review-rust
  - test
dependencies: []
parent_task_id: 'TASK-2241'
modified_files:
  - extensions-terraform/plan/src/lib.rs
priority: medium
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/plan/src/lib.rs:838-1112` (the `#[serial_test::serial(plan_json_max_bytes_env)]` tests)

**What**: Six tests call `unsafe { std::env::set_var(PLAN_JSON_MAX_BYTES_ENV, ...) }` / `remove_var` and are serialized against each other with `#[serial_test::serial(plan_json_max_bytes_env)]`. `serial_test` only excludes tests that carry the same key — it does nothing about tests that carry no annotation at all, and several of those read the process environment concurrently:

- `unexpandable_paths_error_identically_for_out_and_json_file` → `prepare_artifact_paths` / `read_json_file` → `expand_path` → `shellexpand::full` → `std::env::var`
- `artifact_directory_errors_name_the_path_and_the_role` → `prepare_artifact_paths` → same path
- `reserved_passthrough_is_rejected_before_terraform_runs` → `run_terraform_pipeline` → `prepare_artifact_paths` → same path

`setenv` concurrent with `getenv` is a data race on the process `environ` array, which is why Rust 2024 made `set_var` `unsafe` in the first place. The `// SAFETY: serial-style local override; restored at end` comments assert exactly the property that does not hold: serialization is only against the other six.

This is latent under `cargo nextest run` (process-per-test) and live under `cargo test`, which AGENTS.md also lists as part of the gate for doctests and which anyone debugging a single crate reaches for. The failure mode is not a clean assertion failure — it is a torn read or a use-after-free inside libc, so it presents as an intermittent crash or an unrelated test seeing a stale cap.

**Why it matters**: A test suite that is unsound rather than merely flaky produces failures nobody can reproduce or attribute, and the three tests above are the ones covering this crate's path-expansion error contracts — the same contracts TASK-1948 and TASK-1945 were filed to establish. The fix is to stop mutating process state: thread the cap through `read_capped` (and `plan_json_max_bytes()`) as a parameter resolved once at the pipeline boundary, so the tests pass a value instead of setting a variable.

<!-- scan confidence: candidates to inspect -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 No test in this crate mutates the process environment; the byte cap is injected as a value rather than read from env inside read_capped
- [x] #2 plan_json_max_bytes() is resolved once at the pipeline entry point and passed down, keeping the OPS_PLAN_JSON_MAX_BYTES override behaviour and its error message unchanged
- [x] #3 The serial_test dependency and the unsafe set_var/remove_var blocks are removed from this crate's tests, and the existing cap assertions still pass under both cargo test and cargo nextest run

<!-- AC:END -->
