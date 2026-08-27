---
id: TASK-1963
title: >-
  TEST-6: no test covers any failure or hostile-input path of a tool that
  rewrites the user's files in place
status: Triage
assignee: []
created_date: '2026-08-27 15:52'
labels:
  - code-review-rust
  - tests
dependencies: []
modified_files:
  - extensions/text-fixers/src/lib.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Severity**: Medium

**File**: `extensions/text-fixers/src/lib.rs:163-245` (the `tests` module)

**What**: the crate's three end-to-end tests are all happy-path: a dirty file, a clean file, and a small binary fixture, all in a writable tempdir with `tracked = false`. TEST-6 asks for error paths and edge cases; for a tool whose failure mode is corrupting source files, the untested set is the interesting one:

- **Write failure.** No test makes a file or its directory read-only, so the `?` on lib.rs:144 has never executed. Its consequences (no path in the message, the report of already-changed files discarded) are the ERR-13 finding and would have been caught here.
- **Read failure.** No test makes a file unreadable, so the silent `continue` on lib.rs:129-131 has never executed (the ERR-1 finding).
- **Permission preservation.** Nothing chmods a fixture and asserts the mode survives the rewrite. `fs::write` happens to preserve it today; any move to a temp-file-and-rename scheme (required by TASK-1943) silently would not, and no test would notice.
- **Symlinks.** `walk` drops them by design (discovery.rs:70-71) and `tracked_files` does not (TASK-1947), and no test asserts either way.
- **Binary past the sniff window.** `binary.rs` unit-tests the 8 KiB boundary in isolation, but no end-to-end test feeds `run_fixer` a file whose NUL sits past 8 KiB — which is the case where a real file is corrupted (TASK-1951).
- **Large files.** Nothing bounds or exercises input size (TASK-1959).
- **Empty file, and a file that is only whitespace.** `fix_eof` returns `None` for empty input (eof.rs:6-8) and the unit test covers that, but nothing asserts that `run_fixer` leaves a zero-byte file at zero bytes on disk.
- **Idempotence at the run level.** `second_run_is_clean` (line 220) covers one fixture with one dirty line; there is no test that running both fixers twice over a mixed tree (CRLF, no-trailing-newline, dotfiles, nested dirs) is a fixed point. That is the property the exit-code contract depends on: a non-idempotent fixer makes `ops verify` permanently unable to pass, which the discovery module doc (lines 10-15) already identifies as the reason gitignore filtering exists.

**Why it matters**: every finding filed against this crate is in code the current suite does not reach. The tests confirm the fixers do the right thing to a well-behaved file; they say nothing about what happens to a file the fixer should not have touched, which is the whole risk surface of an in-place rewriter on a pre-commit path.

**Suggested fix**: add the failure-path and hostile-input tests above. Several are one-liners over `tempfile::tempdir()` plus `std::fs::set_permissions`. The idempotence one is best expressed as a fixed-point assertion over a fixture tree: run both fixers, snapshot the tree, run both again, assert byte-identical and `!changed()`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A test covers a read-only target file and pins the observable behaviour of the write failure
- [ ] #2 A test covers an unreadable source file and pins the observable behaviour
- [ ] #3 A test chmods a fixture and asserts the file mode is unchanged after a fix
- [ ] #4 A test asserts a zero-byte file is left at zero bytes by both fixers
- [ ] #5 A fixed-point test runs both fixers twice over a mixed fixture tree and asserts the second run changes nothing and reports changed() as false
<!-- AC:END -->
