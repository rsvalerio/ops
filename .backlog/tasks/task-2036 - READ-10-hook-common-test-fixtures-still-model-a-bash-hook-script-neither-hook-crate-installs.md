---
id: TASK-2036
title: >-
  READ-10: hook-common test fixtures still model a bash hook script neither hook
  crate installs
status: Triage
assignee: []
created_date: '2026-08-28 23:24'
labels:
  - code-review-rust
  - tests
dependencies: []
modified_files:
  - extensions/hook-common/src/fixtures.rs
  - extensions/hook-common/src/install.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/hook-common/src/fixtures.rs:15`, `extensions/hook-common/src/fixtures.rs:30`

**What**: Both synthetic `HookConfig` fixtures carry
`hook_script: "#!/usr/bin/env bash\nexec ops run-before-{commit,push}\n"`. Neither wrapper
crate installs a bash script any more — `run-before-push` moved to `#!/bin/sh` with a
`command -v ops` guard in TASK-1911, and `run-before-commit` did the same in TASK-1910. The
fixtures are the last place in the tree that models the shape both crates deliberately
abandoned, so a reader checking "what does an ops hook look like?" from the shared crate's
tests gets the pre-fix answer.

**Why it matters**: READ-10 / TEST — a fixture that has drifted from every real value it
stands in for is a false reference. It is cosmetic today (the install/config tests only need
*some* script string), which is exactly why it will keep drifting.

**Not a one-line change.** `install::tests::classify_existing_hook_separates_partial_from_foreign`
and `install_hook_replaces_truncated_hook_rather_than_calling_it_foreign` (TASK-1882,
TASK-1884) hardcode `"#!/usr/bin/env bash\nexec ops run-before-com"` as a *strict prefix* of
`commit_config().hook_script`, and `classify_existing_hook` classifies by that prefix
relation. Changing the fixture shebang alone reclassifies that literal from `Partial` to
`Foreign` and fails the test — confirmed during TASK-2009, where the fixture edit was
attempted, broke integration verify, and was reverted rather than patched under a held merge
lock. The fix is to derive the truncated literal from `cfg.hook_script` instead of
hardcoding it, then change the fixture.

**Origin**: discovered during TASK-2009 while fixing TASK-1910; the fixture edit was made,
failed integration verify against the sibling wave's tests, and was reverted before the merge.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Both fixture hook_script values use #!/bin/sh, matching what the wrapper crates actually install
- [ ] #2 The truncated-prefix literals in install.rs's Partial-classification tests are derived from cfg.hook_script rather than hardcoded, so a future fixture edit cannot silently reclassify them
<!-- AC:END -->
