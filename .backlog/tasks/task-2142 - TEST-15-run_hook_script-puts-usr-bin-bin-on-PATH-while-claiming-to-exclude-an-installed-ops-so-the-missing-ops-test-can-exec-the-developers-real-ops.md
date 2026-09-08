---
id: TASK-2142
title: >-
  TEST-15: run_hook_script puts /usr/bin:/bin on PATH while claiming to exclude
  an installed ops, so the missing-ops test can exec the developer's real ops
status: To Do
assignee:
  - TASK-2241
created_date: '2026-09-08 07:02'
updated_date: '2026-09-08 10:56'
labels:
  - code-review-rust
  - test-quality
dependencies: []
modified_files:
  - extensions/run-before-push/src/lib.rs
priority: medium
ordinal: 58000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/run-before-push/src/lib.rs:213` (`run_hook_script`), `extensions/run-before-push/src/lib.rs:319` (`hook_script_fails_closed_with_diagnostic_when_ops_is_missing`)

**What**: The harness builds the child PATH as `format!("{}:/usr/bin:/bin", path.display())` under a doc comment claiming "the ambient PATH is deliberately excluded so a developer's installed `ops` cannot satisfy the 'ops missing' case". `/usr/bin` and `/bin` *are* ambient: a distro package, `cargo install --root /usr/local`, or a CI image that copies the built binary into `/usr/bin` all put `ops` exactly there. When that holds:

- `hook_script_fails_closed_with_diagnostic_when_ops_is_missing` writes no fake `ops` into the tempdir, so `command -v ops` resolves the *real* one, the guard is never reached, and the three stderr assertions fail — or pass for the wrong reason if the real ops happens to write "ops" to stderr and exit non-zero.
- Worse, the script falls through to `exec ops run-before-push </dev/null`, so the test **spawns the developer's real ops binary** — a real config load and a real configured command chain — with `OPS_PRE_PUSH_REFS` set from the test's stdin.

The comment's own intent is right; the implementation contradicts it. The missing-ops path needs no external command at all (`command -v` and `echo` are `sh` builtins and the guard exits before `cat`), so that test can run with PATH pointing only at the empty tempdir. Only the fall-through test at :294 needs `cat`, and it can name a directory holding just the tools it needs.

This is the same defect TASK-2113 records for `extensions/run-before-commit/src/lib.rs`; it is filed separately here because the file, the harness function and the test are this crate's own and neither task's fix reaches the other.

**Why it matters**: The test is environment-dependent in the direction that inverts its purpose — green on machines where the guard is untested, and capable of launching an unbounded side-effecting subprocess on exactly those machines. A guard whose test silently stops testing it is how the divergence recorded in TASK-2108 survives.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The missing-ops test makes the ops lookup fail unconditionally (PATH pointing only at an empty temp dir, or empty) rather than relying on /usr/bin and /bin not containing ops
- [ ] #2 No test in this crate can reach 'exec ops run-before-push' with a real ops binary; the fall-through path resolves only the fake ops written by fake_ops
- [ ] #3 The run_hook_script doc comment describes the PATH it actually builds
- [ ] #4 The test still asserts a non-success exit and the ops / hook-path / bypass-variable substrings on stderr, and passes on a machine with ops installed in /usr/bin
<!-- AC:END -->
