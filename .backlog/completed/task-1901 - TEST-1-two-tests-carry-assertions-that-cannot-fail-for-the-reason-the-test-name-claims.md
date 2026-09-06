---
id: TASK-1901
title: >-
  TEST-1: two tests carry assertions that cannot fail for the reason the test
  name claims
status: Done
assignee:
  - TASK-1999
created_date: '2026-08-27 15:38'
updated_date: '2026-08-28 21:22'
labels:
  - code-review-rust
  - test-quality
dependencies: []
modified_files:
  - extensions-rust/metadata/src/tests/accessors.rs
  - extensions-rust/metadata/src/tests/wiring.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/tests/accessors.rs:335-343` and `extensions-rust/metadata/src/tests/wiring.rs:52-63`

**What**: Two separate tests whose assertion does not exercise the behaviour they are named for.

**1. `metadata_max_bytes_is_memoised` (accessors.rs:335)** — cited as "PERF-3 / TASK-1248 AC #3":

```rust
let a = crate::metadata_max_bytes();
let b = crate::metadata_max_bytes();
assert_eq!(a, b, "cached metadata_max_bytes must not drift between calls");
```

Two calls to a function that reads a process-global env var and applies a deterministic parse return the same value whether or not the `OnceLock` exists. Delete the `OnceLock` and this test still passes; that is the definition of a tautology. The property actually worth pinning is that the value is *snapshotted* — that a change to `OPS_METADATA_MAX_BYTES` after the first call does not take effect — and that needs either a `#[serial]`-style env mutation or a seam that lets the snapshot be observed. This crate has already removed one tautology test for exactly this reason: see the note at `ingestor.rs:411-418` ("it exercised no production code path ... gave reviewers false confidence, so it has been removed").

**2. `check_metadata_output_success` (wiring.rs:52)** — the entire assertion sits behind `#[cfg(unix)]` while the test itself is unconditional:

```rust
#[test]
fn check_metadata_output_success() {
    let output = Output { status: std::process::ExitStatus::default(), stdout: vec![], stderr: vec![] };
    #[cfg(unix)]
    assert!(check_metadata_output(&output).is_ok());
}
```

On any non-unix target this compiles to setup with no assertion — a test that cannot fail. Its two siblings (`check_metadata_output_failure_includes_exit_code`, `..._signal_kill_says_signal`) put `#[cfg(unix)]` on the `fn` instead, which is the correct shape: the test simply does not exist off-unix rather than existing and asserting nothing.

**Why it matters**: TEST-1. Both are load-bearing in the sense that a reviewer reading the test list sees "memoisation is covered" and "the success path is covered", and neither is true in the way the name implies. The first also carries an AC reference, so it reads as satisfying a prior task's contract that it does not satisfy.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 metadata_max_bytes_is_memoised either pins the snapshot property (a post-first-call env change is ignored) or is removed with a note, following the precedent at ingestor.rs:411-418
- [x] #2 check_metadata_output_success moves #[cfg(unix)] onto the fn, matching its two siblings, so no target compiles a test body with zero assertions
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
AC #1: metadata_max_bytes_is_memoised removed (it went with tests/accessors.rs under TASK-1898) and the removal is recorded as a note at the end of tests/payload_cap.rs, following the ingestor.rs:411-418 precedent. It is deliberately not reinstated: the snapshot property is unobservable without mutating process-global env, and TASK-1897 now covers every branch of the cap resolution through an injectable seam (tests/payload_cap.rs::max_bytes_env). AC #2: check_metadata_output_success carries #[cfg(unix)] on the fn, matching its two siblings, so no target compiles a test body with zero assertions.
<!-- SECTION:NOTES:END -->
