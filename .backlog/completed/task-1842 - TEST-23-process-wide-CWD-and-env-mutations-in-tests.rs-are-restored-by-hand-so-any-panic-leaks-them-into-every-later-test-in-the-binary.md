---
id: TASK-1842
title: >-
  TEST-23: process-wide CWD and env mutations in tests.rs are restored by hand,
  so any panic leaks them into every later test in the binary
status: Done
assignee:
  - TASK-1997
created_date: '2026-08-27 15:23'
updated_date: '2026-08-28 20:34'
labels:
  - code-review-rust
  - test-quality
dependencies: []
modified_files:
  - extensions-rust/deps/src/tests.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/tests.rs:33-51` (`build_user_context_loads_stack_from_local_ops_toml`), `:320-357` (`check_tool_in_times_out_on_hung_probe`)

**What**: two tests mutate process-global state and restore it with a plain statement on the happy path only. Neither uses a `Drop` guard, so any panic — including the assertion failure the test exists to produce — skips the restore.

```rust
// build_user_context_loads_stack_from_local_ops_toml
let prev = std::env::current_dir().expect("cwd");
std::env::set_current_dir(dir.path()).expect("chdir");
let ctx = build_user_context().expect("build_user_context");   // <-- panics here leave CWD in the tempdir
std::env::set_current_dir(&prev).expect("restore cwd");
```

```rust
// check_tool_in_times_out_on_hung_probe
unsafe { std::env::set_var("CARGO", &fake) };
unsafe { std::env::set_var(ops_core::subprocess::TIMEOUT_ENV, "1") };
let result = check_tool_in(&tool, dir.path());                 // <-- panics here leave CARGO pointed at the fake
unsafe { std::env::remove_var(ops_core::subprocess::TIMEOUT_ENV) };
unsafe { std::env::remove_var("CARGO") };
```

The damage is not confined to the failing test, and that is what makes this worth fixing rather than tolerating:

- **CWD leak**: `tempfile::TempDir` deletes the directory on drop, so after the leak the whole test binary is running in a **deleted** working directory. Every later test that touches a relative path — and `check_tool` at `lib.rs:79-81` hardcodes `std::path::Path::new(".")` — then fails for reasons unrelated to what it tests. Diagnosing that from CI output means correlating two unrelated failures.
- **`CARGO` leak**: the fake `cargo` is `#!/bin/sh\nexec sleep 30`. `ops_core::subprocess::run_cargo` resolves `$CARGO` to keep nested invocations on the parent toolchain, so a leaked `CARGO` redirects *every* subsequent cargo spawn in the process to a script that sleeps for 30 seconds. Combined with a leaked `OPS_SUBPROCESS_TIMEOUT_SECS=1`, later tests get timeout errors that look like real product bugs.

`#[serial]` / `#[serial_test::serial]` prevents *concurrent* corruption but does nothing about *sequential* leakage — it guarantees ordering, not cleanup. The env mutation is additionally `unsafe` under the 2024 edition, and its `// SAFETY:` comment argues only about concurrency, not about the restore being skippable.

Both restores should move into a scope guard whose `Drop` runs on the unwind path (a small local RAII struct, or a crate like `temp-env` / `scopeguard`), so the mutation is reverted whether the body returns or panics. `check_tool_in_times_out_on_hung_probe` also asserts *after* its manual cleanup specifically to work around this — a guard removes the need to order the code that way.

**Why it matters**: a single assertion failure in either test converts a clear one-line CI failure into a cascade of unrelated failures across the rest of the crate's suite, with the true cause several tests upstream. Test infrastructure that only cleans up when the test passes is exactly backwards: the failure path is the one that needs it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 build_user_context_loads_stack_from_local_ops_toml restores the previous working directory through a Drop guard, so an early panic cannot leave the process in the deleted tempdir
- [x] #2 check_tool_in_times_out_on_hung_probe restores/removes CARGO and OPS_SUBPROCESS_TIMEOUT_SECS through a Drop guard covering the unwind path, including when check_tool_in itself panics
- [x] #3 The guard restores the prior value of each env var (including 'was unset') rather than unconditionally removing it
- [x] #4 Assertions can move back above the cleanup code, since cleanup no longer depends on reaching the end of the test body
- [x] #5 The remaining unsafe env blocks carry a SAFETY comment that covers restore-on-unwind, not only the serial/concurrency argument
- [x] #6 Both tests still pass and still carry their #[serial] attributes
<!-- AC:END -->
