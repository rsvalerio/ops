---
id: TASK-1888
title: >-
  TEST-11: upgrade_legacy_hook_bails_if_file_replaced_after_initial_check
  asserts on a temp-file name randomised staging can never produce
status: Done
assignee:
  - TASK-2008
created_date: '2026-08-27 15:34'
updated_date: '2026-08-28 23:06'
labels:
  - code-review-rust
  - tests
dependencies: []
modified_files:
  - extensions/hook-common/src/install.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/hook-common/src/install.rs:649-651`

**What**: The test closes with:

```rust
// Temp file is cleaned up.
let tmp = hooks.join(".pre-commit.ops-tmp");
assert!(!tmp.exists(), "temp file should be removed on bail");
```

`.pre-commit.ops-tmp` is the **pre-TASK-1210 fixed sibling name**. Since the switch to `tempfile::Builder::new().prefix(&format!(".{file_name}.ops-tmp."))` (`install.rs:184-186`), staged files are always named `.pre-commit.ops-tmp.<random>`, so the exact path this assertion probes can never be created by the code under test. The assertion holds unconditionally and verifies nothing: delete the `inspect_err` cleanup, delete `NamedTempFile`'s Drop unlink, leak every stage — the test still passes.

The two sibling tests in the same module already do this correctly, counting directory entries by prefix:

```rust
.filter(|n| n.starts_with(".pre-commit.ops-tmp."))
.count();
assert_eq!(stray, 0, ...);
```

(`upgrade_legacy_hook_ignores_legacy_fixed_name_orphan:701-710` and `upgrade_legacy_hook_concurrent_callers_do_not_corrupt_install:791-800`.)

**Why it matters**: this is the *only* test covering stage cleanup on the bail path — the branch where `upgrade_legacy_hook` refuses because the hook changed under it. That path is the one that runs when a user edits their hook mid-install, and a leaked stage there is a `.pre-commit.ops-tmp.*` orphan sitting inside `.git/hooks/` (executable, since `set_hook_executable` runs on the stage before the recheck). The test's comment claims that leak is covered; the assertion does not cover it. Same TEST-11 shape as the two prefix-counting siblings — fix by matching them.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The bail-path test counts directory entries whose name starts with .pre-commit.ops-tmp. instead of probing the obsolete fixed name, matching its two sibling tests
- [x] #2 The assertion is shown to be load-bearing: it fails if the inspect_err cleanup and the NamedTempFile drop are both bypassed
<!-- AC:END -->
