---
id: TASK-1037
title: >-
  TEST-11: from_env_amortises_tmpdir asserts string equality, not the Arc
  identity it claims to pin
status: Done
assignee: []
created_date: '2026-05-07 20:25'
updated_date: '2026-05-07 23:11'
labels:
  - code-review-rust
  - test
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/expand.rs:362-373`

**What**: The test docstring says: *"constructing Variables::from_env many times must amortise to the cached TMPDIR lookup rather than re-running the std::env::temp_dir() syscall on every call. Pins the OnceLock optimisation; if it regresses (TMPDIR resolved per call) the syscall cost becomes visible at scale."*

The test body, however, only does:

```rust
let warm_tmpdir = warm.builtins.get("TMPDIR").cloned();
for _ in 0..1000 {
    let v = Variables::from_env(&root);
    assert_eq!(v.builtins.get("TMPDIR").cloned(), warm_tmpdir);
}
```

`Arc<str>` compares by *value*, not by pointer, under `PartialEq`. So the assertion holds even if `from_env` were rewritten to call `std::env::temp_dir().display().to_string()` on every invocation and skip the OnceLock entirely — the rendered TMPDIR would be the same string and `assert_eq!` would still pass. The "syscall amortisation" contract the test claims to pin is not actually verified here.

The real Arc-identity assertion lives one screen up at line 196 (`from_env_reuses_cached_tmpdir_arc`) using `Arc::ptr_eq`. That test does the right thing. This 1000-iteration companion adds runtime to `cargo test` while asserting nothing the prior test does not already guarantee — a TEST-11 ("Assert specific values, not just .is_ok() / .is_some()") instance: it asserts a generic equality where the meaningful assertion is pointer identity.

**Why it matters**: A test whose name advertises a stronger guarantee than its body checks is worse than no test — future contributors trust the name and skip writing the actual regression guard. Either delete this test (the Arc::ptr_eq one already covers the contract) or strengthen it to use `Arc::ptr_eq` against `warm.builtins["TMPDIR"]` inside the loop so a regression that swaps OnceLock for "render fresh string each call" actually breaks it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either remove `from_env_amortises_tmpdir` (`from_env_reuses_cached_tmpdir_arc` already pins the contract) or replace its `assert_eq!` with an `Arc::ptr_eq` check that fails when `from_env` re-allocates the TMPDIR string per call
<!-- AC:END -->
