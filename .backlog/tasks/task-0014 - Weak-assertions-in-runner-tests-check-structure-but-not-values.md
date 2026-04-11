---
id: TASK-0014
title: Weak assertions in runner tests check structure but not values
status: Done
assignee: []
created_date: '2026-04-10 18:00:00'
updated_date: '2026-04-11 09:55'
labels:
  - rust-test-quality
  - TQ
  - TEST-11
  - low
  - crate-runner
dependencies: []
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Location**: `crates/runner/src/command/tests.rs:694-700`, `crates/runner/src/command/tests.rs:728-737`
**Anchor**: `fn build_command_with_relative_cwd`, `fn build_command_with_special_chars_in_args`
**Impact**: Several tests assert structural properties (`is_some()`, collection length) without verifying actual values. This weakens their ability to catch regressions where the structure is correct but content is wrong.

**Notes**:
- `build_command_with_relative_cwd` (line 694): asserts `current_dir.is_some()` but does not verify the resolved path value. Should assert the specific expected path.
- `build_command_with_special_chars_in_args` (line 728): asserts `args.len() == 3` but does not verify that special characters (spaces, single quotes, double quotes) are preserved in the argument values. Should assert each arg matches the input.
- Compare with `build_command_with_absolute_cwd` (line 702) which correctly asserts the exact path value — the relative-cwd test should follow the same pattern.
<!-- SECTION:DESCRIPTION:END -->
