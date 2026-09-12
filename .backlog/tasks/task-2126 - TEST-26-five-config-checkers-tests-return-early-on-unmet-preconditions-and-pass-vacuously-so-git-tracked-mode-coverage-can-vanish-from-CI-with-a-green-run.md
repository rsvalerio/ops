---
id: TASK-2126
title: 'TEST-26: five config-checkers tests return early on unmet preconditions and pass vacuously, so git-tracked-mode coverage can vanish from CI with a green run'
status: Done
assignee: []
created_date: '2026-09-08 06:55'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-rust
  - tests
dependencies: []
parent_task_id: 'TASK-2237'
modified_files:
  - extensions/config-checkers/src/tests.rs
priority: medium
ordinal: 42000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/config-checkers/src/tests.rs:186-261, 150-184`

**What**: five tests silently `return` when a precondition is absent and are
reported as **passed**:

- `tracked_only_validates_the_files_git_lists` (`tests.rs:192-194`) — `if !stage_all(root) { return; }`
- `tracked_but_deleted_file_is_skipped_rather_than_failing_the_hook` (`tests.rs:211-213`) — same
- `tracked_symlink_to_a_character_device_is_never_a_candidate` (`tests.rs:231-233` and `tests.rs:248-250`) — returns if `/dev/zero` is absent **and** if `stage_all` fails
- `unreadable_file_is_reported_as_a_read_failure_not_a_parse_failure` (`tests.rs:157-159`) — returns when `is_root_euid()`

`stage_all` (`tests.rs:29-39`) collapses every git failure into `false`:
`git init` missing, `git add` failing on an empty `user.email`/`safe.directory`
config, a sandbox without a `HOME` — all indistinguishable from "git works and
the assertion holds". The helper's own doc comment names the hazard ("any
tracked-mode assertion would pass vacuously — callers must bail out"), but the
bail-out is a bare `return`, which leaves no trace anywhere.

**Why it matters**: `--tracked` is the mode the pre-commit hook actually runs
in, and it is the mode with the security-relevant behaviour — three of these
tests are the regression coverage for TASK-1811 (symlinked character device),
TASK-1813 (tracked-but-deleted file) and the git-index candidate set. A CI
image that stops shipping git, or one whose runner user makes `git add` fail
under `safe.directory`, deletes all of that coverage and the suite still
reports green. Nothing in the test output says the assertions never ran, so
the regression would be found by a user, not by CI.

The root-euid guard has the same shape but a different weight: containers
routinely run tests as root, so `unreadable_file_...` is plausibly *never*
executed in CI while appearing to pass everywhere.

<!-- scan confidence: verified — every listed site read in full -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Missing git no longer produces a silently-passing test: either the tracked-mode tests fail when git is unavailable in an environment that is expected to have it, or the skip is surfaced (eprintln! with the reason plus a #[ignore]-style marker) so a reader of CI output can see the assertion did not run
- [x] #2 stage_all distinguishes 'git binary absent' from 'git command failed', and a git command that fails for a reason other than absence fails the test rather than skipping it
- [x] #3 The /dev/zero and root-euid guards are surfaced the same way, so a run where they trip is distinguishable from one where the assertions executed
- [x] #4 CI is verified to actually execute the tracked-mode tests (they are not skipped in the pipeline that gates merges)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented via shared helpers in ops_core::test_utils (skip_precondition + git_fixture/git_fixture_os classifying BinaryAbsent vs CommandFailed): stage_all now panics on a present-but-refusing git and surfaces a "skip:" line only for a genuinely absent binary; the root-euid and /dev/zero guards print the same marker. Skip surfacing verified by running the compiled test binary with PATH stripped of git. AC #4 verified: gating CI (.github/workflows/ci.yml) runs ubuntu-latest with cargo nextest run --all --all-features — git preinstalled, non-root runner user, /dev/zero present, no #[ignore] on these tests, so the tracked-mode suite executes its assertions in the pipeline that gates merges. Same helper shape reused for TASK-2166 in ops-text-fixers.
<!-- SECTION:NOTES:END -->
