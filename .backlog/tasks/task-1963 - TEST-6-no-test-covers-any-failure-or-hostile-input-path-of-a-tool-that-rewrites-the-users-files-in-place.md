---
id: TASK-1963
title: >-
  TEST-6: no test covers any failure or hostile-input path of a tool that
  rewrites the user's files in place
status: Done
assignee:
  - TASK-2011
created_date: '2026-08-27 15:52'
updated_date: '2026-08-28 23:38'
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
- [x] #1 A test covers a read-only target file and pins the observable behaviour of the write failure
- [x] #2 A test covers an unreadable source file and pins the observable behaviour
- [x] #3 A test chmods a fixture and asserts the file mode is unchanged after a fix
- [x] #4 A test asserts a zero-byte file is left at zero bytes by both fixers
- [x] #5 A fixed-point test runs both fixers twice over a mixed fixture tree and asserts the second run changes nothing and reports changed() as false
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Fixed in TASK-2011. `extensions/text-fixers/src/tests.rs` is now the failure-and-hostile-input suite:

- AC#1 (**substitution**): a read-only *target file* no longer fails. The move to temp-file-and-rename (TASK-1943) means the write only needs a writable directory, and the mode is carried onto the new inode. `a_read_only_target_is_still_rewritten_with_its_mode_intact` pins that changed behaviour explicitly, and `a_failing_write_names_the_path_keeps_going_and_leaves_the_file_intact` pins the write-failure path proper via a read-only *directory* — the file that cannot be written is named, the original is byte-identical afterwards, and the run continues and still reports the file it did fix.
- AC#2 `an_unreadable_file_is_reported_rather_than_making_the_run_look_clean`
- AC#3 `file_mode_survives_the_rewrite` (0640) plus `atomic::tests::preserves_mode`
- AC#4 `a_zero_byte_file_stays_zero_bytes`
- AC#5 `both_fixers_over_a_mixed_tree_reach_a_fixed_point` — LF, CRLF, no-trailing-newline, a dotfile, a nested directory and an empty file; both fixers run twice and the second pass must report `!changed()` and leave the tree byte-identical (snapshot compared)
- also `a_binary_payload_past_the_old_sniff_window_is_left_byte_identical`, `a_file_over_the_cap_is_skipped_reported_and_counted`, `every_discovered_file_is_accounted_for`, and the tracked-mode and symlink cases

All permission-based fixtures go through `test_support` guards that *probe* rather than assume: each returns `None` when the process can defeat the chmod (i.e. root in a CI container), and the test skips instead of asserting something the environment cannot make true.
<!-- SECTION:NOTES:END -->
