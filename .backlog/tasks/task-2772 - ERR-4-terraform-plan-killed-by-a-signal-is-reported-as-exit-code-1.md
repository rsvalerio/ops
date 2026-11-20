---
id: TASK-2772
title: 'ERR-4: terraform plan killed by a signal is reported as exit code 1'
status: Done
assignee: []
created_date: '2026-11-18 18:43'
updated_date: '2026-11-18 22:56'
labels:
  - code-review-rust
  - ERR
dependencies: []
parent_task_id: 'TASK-2856'
modified_files:
  - extensions-terraform/plan/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'ERR-4:extensions-terraform/plan/src/lib.rs:plan_status_result'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/plan/src/lib.rs:825` (`plan_status_result`)

**What**: `bail!("terraform plan failed with exit code {}", code.unwrap_or(1))` makes up an exit status when `ExitStatus::code()` is `None`, which on unix means the child was killed by a signal (OOM killer, SIGTERM from a CI runner, a SIGKILL). The operator is told terraform exited 1, which is terraform's ordinary "error" code, and the real cause is lost. The test `plan_status_interpretation_matches_terraform_exit_codes` pins this fallback. The sibling `show_failure_error` (`lib.rs:889`) already handles the same case correctly ("was terminated by a signal").

**Why it matters**: The error context is wrong, not just missing. A CI log showing "exit code 1" sends the operator looking for a terraform diagnostic that does not exist, when the real cause is resource exhaustion or an external kill. The two terraform invocations in this pipeline also report the same condition in different ways.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A None exit code yields an error saying terraform plan was terminated by a signal (naming the signal via ExitStatusExt::signal on unix where available), not a fabricated exit code
- [x] #2 plan_status_interpretation_matches_terraform_exit_codes is updated to assert the signal wording

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.21 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
