---
id: TASK-1908
title: >-
  TEST-5: has_staged_files (the production preflight) has zero coverage — every
  test goes through a cfg(test)-only shim that bypasses the cwd and env-timeout
  wiring
status: To Do
assignee:
  - TASK-2009
created_date: '2026-08-27 15:39'
updated_date: '2026-08-28 14:17'
labels:
  - code-review-rust
  - tests
dependencies: []
modified_files:
  - extensions/run-before-commit/src/lib.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/run-before-commit/src/lib.rs:79-96`

**What**: The function production registers as the hook preflight (`crates/cli/src/pre_hook_cmd.rs:16`, `preflight: Some((ops_run_before_commit::has_staged_files, "no staged files"))`) is:

```rust
pub fn has_staged_files() -> anyhow::Result<bool> {
    let cwd = std::env::current_dir().context("failed to read current directory")?;
    let timeout = git_timeout_from_env().unwrap_or(DEFAULT_GIT_TIMEOUT);
    has_staged_files_with_timeout("git", &cwd, timeout).map_err(anyhow::Error::from)
}
```

No test in the workspace calls it. `grep -rn "has_staged_files\b"` finds the definition, the `pre_hook_cmd` registration, and nothing else. Every one of the seven `has_staged_files_*` tests in this file (lines 228-385) instead calls one of:

```rust
#[cfg(test)]
fn has_staged_files_with(program: &str, dir: &Path) -> Result<bool, HasStagedFilesError> {
    has_staged_files_with_timeout(program, dir, DEFAULT_GIT_TIMEOUT)
}
```

...or `has_staged_files_with_timeout` directly. Both take `program` and `dir` as parameters, so the shim is a *divergent* double: it exercises the shared `ops_hook_common::git_state` probe (already covered by that crate's own tests) while skipping every line this crate actually contributes:

- `std::env::current_dir()` + its `"failed to read current directory"` context
- `git_timeout_from_env().unwrap_or(DEFAULT_GIT_TIMEOUT)` — the env override reaching the probe at all
- the hardcoded `"git"` program name
- `map_err(anyhow::Error::from)` — that a `HasStagedFilesError` survives into the `anyhow` chain the CLI prints

`git_timeout_from_env` is tested in isolation (lines 407-444), but nothing pins that its result is what `has_staged_files` passes down. Swapping `unwrap_or(DEFAULT_GIT_TIMEOUT)` for `unwrap_or_default()` yields a zero-second timeout that fails every commit, and the entire suite still passes. So does dropping the env lookup entirely.

**Why it matters**: TEST-5 — every public API function needs at least one test, and this is the only production entry point the crate contributes beyond macro-generated wrappers. The four lines with no coverage are exactly the four that turn a well-tested generic probe into this hook's behaviour; a regression in any of them lands on the developer's commit path with no test failing. The shim also creates the classic double-divergence trap: the suite looks well covered (seven tests, timeout, deadlock, lossy-UTF-8, missing-binary cases) while the shipped function is untouched.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A test invokes has_staged_files() itself (via a CwdGuard-style temp-repo cwd switch) and asserts true for a staged file and false for an empty index
- [ ] #2 A test asserts that setting OPS_RUN_BEFORE_COMMIT_GIT_TIMEOUT_SECS changes the timeout has_staged_files actually applies, not just what git_timeout_from_env returns
- [ ] #3 A test asserts the error from a non-repo cwd reaches the caller as an anyhow chain whose {:#} rendering names the underlying HasStagedFilesError
- [ ] #4 The cfg(test)-only has_staged_files_with shim is either removed or documented as covering only the shared probe, so it is not mistaken for coverage of the production entry point
<!-- AC:END -->
