---
id: TASK-1082
title: >-
  TEST-12: cli_with_invalid_ops_d_file failure mode is ambiguous (also matches
  unknown-command failures)
status: Done
assignee: []
created_date: '2026-05-07 21:20'
updated_date: '2026-05-08 06:17'
labels:
  - code-review-rust
  - TEST
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `tests/integration.rs:430-448`

**What**: Test runs `ops build` against a malformed `.ops.d/invalid.toml` and asserts only `.failure()`. But `build` is not a declared command in this fixture; failure could equally come from "unknown command" rather than the malformed `.ops.d` file the test is named for. The test name and the assertion don't agree.

**Why it matters**: A regression that started accepting malformed TOML silently would still pass as long as the unknown-command path failed first. The test cannot detect what its name advertises.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Use a defined command name in .ops.toml so the only remaining failure mode is the malformed .ops.d file
- [ ] #2 Assert stderr mentions the malformed file path or 'parse'
- [ ] #3 Add a sibling positive test that the same .ops.toml works without the malformed .ops.d entry
<!-- AC:END -->
