---
id: TASK-1802
title: >-
  TEST-18: chmod 0o000 permission tests fail when the suite runs as root and
  leave the tree unreadable on panic
status: Triage
assignee: []
created_date: '2026-08-27 11:25'
labels:
  - code-review-rust
  - test-quality
dependencies: []
modified_files:
  - extensions-rust/cargo-toml/src/tests/provider.rs
  - extensions-rust/cargo-toml/src/tests/find_root.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/tests/provider.rs:58-92` (`provider_unreadable_file_returns_error`) and `extensions-rust/cargo-toml/src/tests/find_root.rs:273-303` (`find_root_canonicalize_perm_denied_returns_typed_error`).

**What**: both tests make the DAC permission bits the mechanism under test:

```rust
std::fs::set_permissions(&cargo_toml, std::fs::Permissions::from_mode(0o000)).ok();
...
assert!(result.is_err(), "unreadable file should return error");
```

Two problems.

1. **Environment-dependent outcome.** `CAP_DAC_OVERRIDE` (uid 0 — the default in most CI container images, and in `docker run` without `--user`) bypasses the 0o000 bits entirely: the read succeeds, `provide` returns `Ok`, and `assert!(result.is_err())` fails. The same applies to the 0o000 *directory* in the find_root test, where root can still traverse. Neither test declares this precondition, so the failure presents as a mysterious red on a machine that differs only in uid. `provider_unreadable_file_returns_error` also swallows the `set_permissions` result with `.ok()`, so a filesystem that ignores mode bits (a mounted volume with `noacl`-style semantics, some CI overlay/9p mounts, Windows) produces the same unexplained failure rather than a skip.

2. **Cleanup is not panic-safe.** The restore is a plain statement after the call under test:

```rust
let result = provider.provide(&mut ctx);
// ... restore to 0o644 ...
assert!(result.is_err(), ...);
```

In the find_root test the restore is `fs::set_permissions(&locked, ...).unwrap()` placed *before* the assertions, which is correct ordering — but if `find_workspace_root` itself panics, or if any earlier `unwrap()` in the setup fires, the 0o000 directory survives and `tempfile::TempDir`'s `Drop` cannot remove it, leaking an undeletable directory into the temp filesystem for every subsequent run.

**Why it matters**: TEST-18 / flakiness — the outcome depends on ambient process privilege and mount semantics rather than on crate logic, and the failure mode erodes trust in an otherwise well-covered suite. This is a known pattern, not a hypothetical: these are the only two tests in the crate whose result is a function of the uid the runner happens to have.

Fix shape: put the restore in a `Drop` guard so cleanup is unconditional, and make the privilege precondition explicit — probe once (attempt the read after chmod; if it still succeeds, the environment cannot express the condition) and skip with a message, or gate the assertion behind that probe. A `#[ignore = "requires a non-root uid; run with: cargo test -- --ignored"]` (TEST-24) is an acceptable alternative if the probe is judged not worth the code.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Neither test fails when the suite runs as uid 0: the privilege precondition is probed and the test skips (or is documented #[ignore] with a run instruction) instead of asserting an outcome the environment cannot produce
- [ ] #2 Permission restoration happens in a Drop guard so a panic anywhere in the test body still leaves the tempdir removable
- [ ] #3 The set_permissions result is no longer discarded with .ok() where the test's correctness depends on it having taken effect
- [ ] #4 Both tests still assert the typed error (DataProviderError::ComputationFailed / FindWorkspaceRootError variant) on an environment where the permission bits do apply
<!-- AC:END -->
