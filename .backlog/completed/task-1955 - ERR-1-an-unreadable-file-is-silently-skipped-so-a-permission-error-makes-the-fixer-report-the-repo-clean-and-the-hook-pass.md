---
id: TASK-1955
title: >-
  ERR-1: an unreadable file is silently skipped, so a permission error makes the
  fixer report the repo clean and the hook pass
status: Done
assignee:
  - TASK-2011
created_date: '2026-08-27 15:50'
updated_date: '2026-08-28 23:37'
labels:
  - code-review-rust
  - idioms
dependencies: []
modified_files:
  - extensions/text-fixers/src/lib.rs
  - extensions/text-fixers/src/discovery.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Severity**: Medium

**File**: `extensions/text-fixers/src/lib.rs:129-131` (`run_fixer`)

**What**:

```
let Ok(bytes) = std::fs::read(&path) else {
    continue;
};
```

The `io::Error` is neither handled nor propagated — it is discarded with no message, no counter, and no effect on the exit code. Note also that the skip happens *before* `report.files_scanned` is incremented (line 133), so the file vanishes from the summary entirely: `write_summary` prints "scanned N file(s), 0 changed" and the user has no way to tell that a file was never looked at.

The same swallow exists one level down in `discovery::walk` (discovery.rs:69), where `walker.flatten()` drops every `ignore::Error` — an unreadable directory silently contributes zero files.

Reachable causes, all mundane: a file mode-600 owned by another user (very common in a container or on a shared build agent), a file removed between discovery and read (the `git ls-files` index lists paths that may not exist in the worktree — a staged deletion is exactly this), a dangling symlink under `--tracked`, EIO, a path that exceeds the OS limit.

**Why it matters**: the crate's contract, stated in the lib.rs module doc, is "the CLI can exit non-zero when at least one file was modified (matches the pre-commit contract that fails a commit on change)". The inverse — exit zero means the tree is clean — is what a hook driver actually relies on. Swallowing read errors makes exit zero mean "clean, or unreadable, we did not distinguish", so a commit passes the hook on files the fixer never examined. That is a fail-open on a gate.

**Suggested fix**: count the skips and surface them. At minimum, a `files_skipped: usize` on `FixerReport` plus a line on the writer naming the path and the error (see the companion ERR-13 finding on path context). Decide deliberately whether a read failure should also make the run exit non-zero — for a gate, "could not check" is usually closer to "failed" than to "passed" — and write that decision into the module doc next to the existing exit-code paragraph.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A file that cannot be read is reported on the writer with its path and the underlying io error, not silently skipped
- [x] #2 FixerReport carries the number of skipped files and write_summary includes it, so scanned + skipped accounts for every discovered path
- [x] #3 The module doc states whether an unreadable file makes the run exit non-zero, and the code matches that statement
- [x] #4 A test makes one fixture unreadable, runs a fixer, and asserts the skip is reported rather than the run looking clean
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Fixed in TASK-2011. The `let Ok(bytes) = fs::read(..) else { continue }` is gone. `runner::read_candidate` classifies every discovered path, and the outcome is always visible:

- read/metadata failure -> `FixerReport::files_failed` with the path, the `io::ErrorKind` and the message, rendered as `<label>: <path>: read: <err>`;
- deliberate skips (over the cap, not a regular file, vanished, not text) -> `FixerReport::files_skipped`.

AC#2 is satisfied in a refined form (**substitution**): the accounting is three-way, not two-way. `files_scanned + files_skipped + files_failed` accounts for every discovered path, and `write_summary` prints all four numbers. An unreadable file is a *failure*, not a skip, because "could not check" on a gate is closer to "failed" than to "passed" — so counting it under `files_skipped` as the AC literally asks would have understated it. `tests::every_discovered_file_is_accounted_for` pins the identity.

AC#3: the lib.rs module doc has an "# Exit-code contract" section stating that the CLI exits non-zero when `changed()` **or** `failed()`; `crates/cli/src/subcommands.rs::run_text_fixer` matches it.

AC#4: `tests::an_unreadable_file_is_reported_rather_than_making_the_run_look_clean` (chmod 0000 fixture, self-skipping under root).

The companion swallow in `discovery::walk` is fixed too: `walker.flatten()` dropped every `ignore::Error`, so an unreadable directory contributed zero files in silence. The walk now collects those into `Discovery::walk_errors` and both consumers print them. Test: `discovery::tests::a_directory_the_walk_cannot_enter_is_reported_not_swallowed`.
<!-- SECTION:NOTES:END -->
