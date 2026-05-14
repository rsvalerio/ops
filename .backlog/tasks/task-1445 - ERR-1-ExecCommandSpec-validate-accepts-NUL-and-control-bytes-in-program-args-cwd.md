---
id: TASK-1445
title: >-
  ERR-1: ExecCommandSpec::validate accepts NUL and control bytes in
  program/args/cwd
status: Done
assignee:
  - TASK-1456
created_date: '2026-05-13 18:44'
updated_date: '2026-05-14 07:41'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/commands.rs:104-114`

**What**: `ExecCommandSpec::validate` only checks empty program and zero timeout. A `program = "\u{0}"` or literal newline in program passes validate and only fails downstream at spawn-time with a generic `EINVAL`. Other foot-guns (unprintable characters, embedded newlines in args, control chars in cwd) also pass — argv elements that survive into `display_cmd`/dry-run via `shell_quote` are quoted but never rejected.

**Why it matters**: Config validation is the single point where bad shapes should fail loud and named. Catching NUL / control bytes at load time turns "spawn fails with cryptic OS error after a hook fires" into "config '<name>': program contains control characters at position N".
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 ExecCommandSpec::validate rejects programs containing \0 or any character < 0x20 (excluding \t) with a named error
- [ ] #2 Same check applies to every element of args and to cwd if Some
- [ ] #3 Regression test for each rejected shape (program, args element, cwd)
<!-- AC:END -->
