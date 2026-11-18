---
id: TASK-2811
title: 'TEST-11: concurrent upgrade test asserts a loser error path the code no longer has'
status: To Do
assignee: []
created_date: '2026-11-18 18:46'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - TEST
dependencies: []
parent_task_id: 'TASK-2861'
modified_files:
  - extensions/hook-common/src/install.rs
priority: low
ordinal: 1000
dedup_key: 'TEST-11:extensions/hook-common/src/install.rs:upgrade_legacy_hook_concurrent_callers_do_not_corrupt_install'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/hook-common/src/install.rs:1118-1189` (`upgrade_legacy_hook_concurrent_callers_do_not_corrupt_install`)

**What**: The test's doc (install.rs:1118-1125) and inline comment (install.rs:1157-1159) say the loser of the rename race "observes the post-win content and returns the typed 'file changed during install' error". `upgrade_legacy_hook` no longer does that: a re-read that sees the current payload is classified `ExistingHook::Current` and returns `Ok` (install.rs:334-338), and a re-read before the winner's rename still sees `Legacy` and persists the same bytes. Every caller therefore succeeds. The assertions accept either error string — one of which, "failed to re-read existing hook", the code never produces (the real message is "failed to read existing hook") — and only require `wins >= 1`.

**Why it matters**: The test cannot fail if a regression makes concurrent honest installs error out again, which is exactly the behaviour the `Current` arm was added to prevent (its comment: "Erroring here would make two honest simultaneous installs fail one of themselves"). The stale doc also misdescribes the contract to the next reader.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The test asserts every concurrent caller returns Ok (wins == n) and the final hook content equals the ops payload
- [ ] #2 The test doc and inline comments describe the Current-on-recheck success path instead of a loser error
- [ ] #3 The never-produced 'failed to re-read existing hook' string is removed from the test
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.27 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
