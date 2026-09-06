---
id: TASK-1957
title: >-
  ERR-13: the write error carries no path and aborts the whole run, discarding
  the report of files already rewritten
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
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Severity**: Medium

**File**: `extensions/text-fixers/src/lib.rs:144` (`run_fixer`)

**What**: `std::fs::write(&path, &fixed)?` propagates a bare `std::io::Error` into `anyhow::Result`. Two defects in one line.

**(a) No path in the message.** The user sees `Permission denied (os error 13)` — from a tool that walked an entire repository and could have been rewriting any of thousands of files. There is nothing in the error chain, and no `.with_context()` anywhere in this crate, to say which one. `run_text_fixer` in `crates/cli/src/subcommands.rs:313` adds no context either, so the bare message is what reaches the terminal and the CI log. This is the textbook ERR-13 case; the crate has exactly one `std::fs` write and one `std::fs` read (line 129), so either `.with_context(|| format!("writing {}", path.display()))` or swapping the module for `fs_err` is a two-line fix.

**(b) The `?` throws away work already done.** `report` at that point holds every file rewritten so far, and it is dropped on the floor when the error propagates. `run_text_fixer` never reaches `write_summary`, so the run ends with an error and *no record of the files it already modified*. The user is left with a partially fixed tree and no list. For a tool that mutates the working copy, "what did you change before you gave up?" is the single most important thing the error path can answer.

There is also a policy question worth settling: one unwritable file (read-only fixture, a file owned by root, a read-only mount under the root) currently stops the fixer from processing the rest of the repo. Continuing and reporting the failures at the end is the usual choice for a batch file rewriter.

**Why it matters**: a destructive tool whose failure mode is "an error with no filename and no record of what it already did" is very expensive to recover from, and the recovery has to happen under time pressure because it is blocking a commit.

**Suggested fix**: attach the path with `.with_context(...)` (or adopt `fs_err`, which folds the path in automatically and also covers the `fs::read` on line 129). Collect per-file failures into the `FixerReport` and keep going, then return an error at the end that names them — so the summary of what *was* changed still prints.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A failing write produces an error message that names the file path, verified by a test asserting the path substring appears in the error chain
- [x] #2 The fs::read on line 129 carries the same path context, or the crate uses fs_err throughout
- [x] #3 A write failure no longer discards the list of files already rewritten: the report of prior changes is still surfaced to the caller
- [x] #4 The decision to abort or continue after a per-file write failure is explicit in the function doc, and the code matches it
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Fixed in TASK-2011.

AC#1 is satisfied by **substitution**, and the substitution is the point of the fix. There is no longer an error chain to inspect for a per-file write failure, because the `?` that discarded the report is gone: a write failure is recorded in `FixerReport::files_failed` (path + `FailureKind::Write(kind)` + message) and rendered on the writer as `<label>: <path>: write: <err>`, and the run continues. `tests::a_failing_write_names_the_path_keeps_going_and_leaves_the_file_intact` asserts the path appears both in the record and in the rendered line — the same guarantee the AC was after, on the surface that now carries it.

AC#2: `runner::read_candidate` renders read and metadata failures with the same path-prefixed line, so the read on the old line 129 carries path context too. `fs_err` was not adopted: the crate has exactly two I/O sites and both now go through `read_candidate` / `atomic::replace`.

AC#3: the report survives. `run_fixer` only aborts on discovery failure or writer failure, so the list of files already rewritten always reaches the caller and `write_summary` always prints.

AC#4: the continue-on-failure decision and its reasoning are in the `run_fixer` doc under "# Per-file failures do not abort the run". The CLI turns `failed()` into a non-zero exit.
<!-- SECTION:NOTES:END -->
