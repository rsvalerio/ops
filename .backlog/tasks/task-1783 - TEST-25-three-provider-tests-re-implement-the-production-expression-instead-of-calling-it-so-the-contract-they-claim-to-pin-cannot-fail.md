---
id: TASK-1783
title: >-
  TEST-25: three provider tests re-implement the production expression instead
  of calling it, so the contract they claim to pin cannot fail
status: To Do
assignee:
  - TASK-1995
created_date: '2026-08-27 11:23'
updated_date: '2026-08-28 14:12'
labels:
  - code-review-rust
  - test-quality
dependencies: []
modified_files:
  - extensions-rust/cargo-update/src/tests.rs
  - extensions-rust/cargo-update/src/lib.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-update/src/tests.rs:101-109`, `:646-678`, `:686-717`

**What**: Three tests state in their doc comments that they pin a contract in
`CargoUpdateProvider::provide` / `parse_action_line`, then verify a locally
rebuilt copy of that expression rather than invoking the production code. Each
would keep passing verbatim if the production site regressed to the exact form
the test exists to forbid.

1. `non_zero_exit_stderr_tail_debug_escapes_control_bytes` (`:646`) — claims to
   pin SEC-21 / TASK-1537 on the non-zero-exit branch of `provide` (`lib.rs:444-460`).
   It never calls `provide`. It builds `let exit_label = "exit status: 101";`
   and formats `"cargo update --dry-run exited with status {exit_label}: {stderr_tail:?}"`
   by hand. Change `lib.rs:458` from `{:?}` back to `{}` and this test still
   passes — the very regression SEC-21 was filed against. Its own comment
   admits the gap: *"the live call would require spawning cargo, so we exercise
   the format expression directly"*.

2. `provide_wraps_run_error_with_context_preserving_source_chain` (`:686`) —
   claims to pin ERR-4 / TASK-1535 on `lib.rs:434-436`. It constructs
   `anyhow::Error::new(underlying).context("cargo update --dry-run failed")`
   itself and asserts `anyhow::Chain` contains a `RunError`. That is a test of
   `anyhow`'s `context` implementation (TEST-25: framework-only). Reverting
   `provide` to `anyhow!("{}: {}", ctx, e)` leaves it green.

3. `warn_breadcrumb_debug_escapes_control_characters` (`:101`) — claims to pin
   ERR-7 / TASK-0975 on the `tracing::warn!(line = ?clean, …)` breadcrumbs.
   Its whole body is `let rendered = format!("{line:?}");` plus assertions —
   a test of `std::fmt::Debug` for `&str`. Switching any `?clean` to `%clean`
   in `lib.rs` leaves it green. The workspace already owns this assertion as
   `ops_about::test_support::assert_debug_escapes_control_chars`
   (`extensions/about/src/test_support.rs:90`), whose module doc records the
   same lesson from TASK-0985.

**Why it matters**: these are the three highest-value contracts in the crate —
the only two hardening fixes on the provider's error path, and the log-forgery
guard on the parser. All three are pinned by copies, so the suite reports
coverage it does not have, and a future refactor of `provide` gets a green
build while re-opening SEC-21 and ERR-4. The root cause is the missing seam
called out separately: `provide` hardcodes `run_cargo_update_dry_run`, so there
is no way to hand it a canned `std::process::Output` (see the TEST-5 finding on
the same file).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 non_zero_exit_stderr_tail_debug_escapes_control_bytes drives the real non-zero-exit branch of CargoUpdateProvider::provide (via an injected Output/runner seam) and fails if the {:?} formatting at lib.rs:458 is reverted to {}
- [ ] #2 provide_wraps_run_error_with_context_preserving_source_chain asserts on the anyhow::Error actually returned by provide, and fails if the .context(..) wrap is flattened back into anyhow!("{}: {}", ..)
- [ ] #3 warn_breadcrumb_debug_escapes_control_characters asserts on a tracing record captured from a real parse_update_output call, and fails if a ?field is changed to %field; the assertion body reuses ops_about::test_support::assert_debug_escapes_control_chars rather than re-deriving it
- [ ] #4 No test in this file constructs a copy of a production format!/anyhow! expression as its subject under test
<!-- AC:END -->
