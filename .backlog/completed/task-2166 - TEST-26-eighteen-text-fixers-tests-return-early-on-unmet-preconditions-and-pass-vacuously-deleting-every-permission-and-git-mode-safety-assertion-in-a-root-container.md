---
id: TASK-2166
title: 'TEST-26: eighteen text-fixers tests return early on unmet preconditions and pass vacuously, deleting every permission and git-mode safety assertion in a root container'
status: Done
assignee: []
created_date: '2026-09-08 07:06'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-rust
  - tests
dependencies: []
parent_task_id: 'TASK-2237'
modified_files:
  - extensions/text-fixers/src/tests.rs
  - extensions/text-fixers/src/discovery/tests.rs
  - extensions/text-fixers/src/atomic.rs
  - extensions/text-fixers/src/test_support.rs
priority: medium
ordinal: 79000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/text-fixers/src/tests.rs`, `extensions/text-fixers/src/discovery/tests.rs`, `extensions/text-fixers/src/atomic.rs`

**What**: eighteen bare `return;` bail-outs on an unmet precondition, each of
which reports the test as **passed**:

Root / permission guards (`ReadOnlyDir`, `UnreadableFile`, `UnsearchableDir`
all return `None` when the chmod does not deny the current process):

- `tests.rs:111` `a_file_whose_write_fails_is_counted_once`
- `tests.rs:286` `a_failing_write_names_the_path_keeps_going_and_leaves_the_file_intact`
- `tests.rs:327` `an_unreadable_file_is_reported_rather_than_making_the_run_look_clean`
- `tests.rs:444` `a_walk_error_reaches_the_report_and_the_summary`
- `discovery/tests.rs:105` `a_directory_the_walk_cannot_enter_is_reported_not_swallowed`
- `atomic.rs:179` `a_failed_replace_leaves_the_original_intact`

git guards (`git_init` / `git_add` / `git_available` / `is_inside_repo`, all of
which collapse every failure into `false`):

- `tests.rs:359`, `tests.rs:388`, `tests.rs:411`, `tests.rs:414`
- `discovery/tests.rs:141`, `:164`, `:193`, `:196`, `:217`, `:236`, `:272`, `:278`

**Why it matters**: this is the same shape as TASK-2126 in the sibling
extension, but with a heavier payload. The six permission-guarded tests are
the *entire* regression suite for this crate's stated purpose — the crate
header says "a mode-600 file (routine in a container or on a shared build
agent) made the run report a clean tree it had never looked at", and
`FixerReport::failed`, `record_failure`, `walk_errors` and the atomic-replace
rollback exist to close that. CI containers routinely run as root, where every
one of those six guards returns `None` and every assertion is skipped while
the suite reports green. The failure the crate was written to prevent has no
executing test in the environment that gates merges.

`test_support.rs` documents the hazard ("Callers treat `None` as 'this hazard
cannot be simulated here' and skip") but the skip leaves no trace in the test
output, so nobody reading a green run can tell the difference between "the
rollback works" and "the rollback was never exercised".

The git helpers add the second failure mode from TASK-2126: `git_os` returns
`false` for a missing binary, a `safe.directory` refusal, an unset
`user.email`, and a sandbox with no `HOME` — all indistinguishable from
success, so the `--tracked` coverage (including
`tracked_mode_never_rewrites_through_a_symlink_out_of_the_root`, the
path-escape test) can vanish silently.

<!-- scan confidence: verified — every listed site read in full -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A test whose precondition is unmet is distinguishable from one that ran: the skip is surfaced in the output with its reason, not a bare return
- [x] #2 The six permission-guarded tests are verified to actually execute in the CI pipeline that gates merges, or CI is changed so they do
- [x] #3 The git helpers distinguish 'git binary absent' from 'git command failed'; a git command failing for any reason other than absence fails the test rather than skipping it
- [x] #4 Resolved consistently with TASK-2126 in config-checkers, since both crates share the same helper shape
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
test_support.rs git helpers (git_init/git_add/git_available) now run through ops_core::test_utils::git_fixture: BinaryAbsent => "skip:" line printed inside the helper + false; CommandFailed => panic (is_inside_repo keeps bool semantics — a refused rev-parse is its expected not-a-repository answer). git_add returns () so the old assert! wrappers are gone (a refusing git fails inside the helper). All six permission guards (tests.rs x4, discovery/tests.rs x1, atomic.rs x1) surface skip_precondition with the fixture name and reason. Verified by running the compiled test binary with PATH stripped of git: "skip: git fixture: git is not on PATH; git-mode assertions did not run" prints while the test passes. AC #2: gating CI is ubuntu-latest under the non-root runner user, so the chmod guards genuinely deny and all six permission-guarded tests execute their assertions there. AC #4: same ops_core::test_utils helpers as TASK-2126.
<!-- SECTION:NOTES:END -->
