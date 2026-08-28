---
id: TASK-1841
title: >-
  TEST-6: every degraded and give-up path in the allocation loop is untested —
  the MAX_ALLOCATION_ATTEMPTS bail, the silent directory skips, and the public
  entry point
status: Done
assignee:
  - TASK-2005
created_date: '2026-08-27 15:23'
updated_date: '2026-08-28 15:53'
labels:
  - code-review-rust
  - test-quality
dependencies: []
modified_files:
  - extensions/create-review-tasks/src/lib.rs
  - extensions/create-review-tasks/src/backlog.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/create-review-tasks/src/lib.rs:93-100`, `232-243`; `extensions/create-review-tasks/src/backlog.rs:140-156`

**What**: The suite is strong on the happy paths and on the races it *can* force (`concurrent_runs_allocate_disjoint_task_sets`, `taken_filename_reallocates_and_leaves_no_partial_set`, `write_task_set_rejects_a_claim_another_number_already_holds`), which makes the gaps below stand out as omissions rather than a generally thin suite. Four reachable branches have no coverage at all:

1. **The give-up bail (lib.rs:238-243).** `commit_task_set` loops `MAX_ALLOCATION_ATTEMPTS = 32` times and then produces the only operator-facing error this function can emit — "gave up allocating a free review-request id after 32 attempts". Nothing exercises it. The loop bound, the message, and the fact that a `StagedTasks` from the final attempt is dropped (rolled back) before the bail are all unpinned. This is testable without concurrency: a fixture that pre-creates the main-task filename for every number the retry sequence will try, or a seam over `MAX_ALLOCATION_ATTEMPTS`, drives it deterministically.

2. **`for_each_task_file`'s unreadable-directory skip (backlog.rs:147-149).** The `let Ok(entries) = read_dir(..) else { continue }` arm is the crate's deliberate "treat an unreadable directory like an absent one" policy, documented at lines 140-144. A directory that exists but denies read (chmod 000) is never tested, so the policy is asserted only in prose — and the consequence is severe: silently skipping a populated `completed/` directory makes `next_main_task_number` hand out an id that already exists, which is exactly the collision the TASK_DIRS comment (lines 15-18) says must never happen.

3. **The non-UTF-8 filename skip (backlog.rs:151).** `entry.file_name().into_string()` discards non-UTF-8 names. Same silent-skip risk, same absence of a test.

4. **`run_create_review_tasks` (lib.rs:93-100), the crate's only public entry point.** Every test calls `run_create_review_tasks_at` instead. The public wrapper — and therefore `UtcStamp::now()`, the one line that differs — is executed by no test in this crate (TEST-5).

Related untested boundaries (TEST-8) in the same functions: `slugify` on non-ASCII input, and a `max` at `u32::MAX` where `saturating_add(1)` returns the same number and the retry loop cannot make progress.

**Why it matters**: Three of the four are *silent-degradation* branches — they do not fail, they return a plausible-looking wrong answer (a skipped directory, a skipped filename, a stamp from a bad clock). Silent degradation is precisely the category that needs a test, because no other signal exists: a run that skips `completed/` still prints `created TASK-…` and still exits 0. The fourth, the give-up bail, is the message an operator will see during a real contention incident, and it is currently unverified prose. Together they are the difference between "the concurrency design is documented" and "the concurrency design is pinned".
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 a deterministic test drives commit_task_set to MAX_ALLOCATION_ATTEMPTS exhaustion and asserts the error names the tasks directory and the attempt count, and that no file from the final abandoned attempt survives
- [x] #2 a test covers for_each_task_file against a TASK_DIRS directory that exists but cannot be read, pinning the documented 'treat as absent' policy
- [x] #3 a test covers a non-UTF-8 filename in a scanned directory and pins that it is skipped rather than panicking or aborting the scan
- [x] #4 run_create_review_tasks (the public entry point) is exercised by at least one test, so the wrapper and UtcStamp::now are not entirely uncovered
- [x] #5 boundary tests exist for slugify on non-ASCII input and for next_main_task_number when the highest observed number is u32::MAX
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Added, all deterministic and single-threaded:

1. `allocation_gives_up_after_max_attempts_and_rolls_back` — a backlog holding
   `task-4294967295 - review-request-2026-08-20-4294967295.md` saturates both maxima,
   so every attempt re-derives that exact filename and loses `create_new`. Asserts
   the error names MAX_ALLOCATION_ATTEMPTS and the tasks directory, that nothing is
   reported, and that the tasks dir still holds only the foreign file. No seam over
   the constant was needed.
2. `unreadable_task_dir_is_skipped_like_an_absent_one` — `chmod 000` on `completed`
   pins the documented "treat as absent" policy. The assertion is guarded by a probe
   read, because root ignores the mode bits and the policy is unobservable there.
3. `non_utf8_file_name_is_skipped_without_aborting_the_scan` — an invalid-UTF-8 entry
   is skipped and the rest of the scan still counts.
4. `public_entry_point_writes_a_set_dated_by_the_host_clock` — exercises
   `run_create_review_tasks` and therefore `UtcStamp::now`.
5. `next_number_saturates_at_u32_max` plus
   `slugify_and_file_names_pin_non_ascii_titles` cover the TEST-8 boundaries.
<!-- SECTION:NOTES:END -->
