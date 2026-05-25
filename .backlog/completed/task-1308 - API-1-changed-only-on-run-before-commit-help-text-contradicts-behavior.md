---
id: TASK-1308
title: 'API-1: --changed-only on run-before-commit help text contradicts behavior'
status: Done
assignee:
  - TASK-1382
created_date: '2026-05-11 19:58'
updated_date: '2026-05-12 22:59'
labels:
  - code-review-rust
  - API
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/args.rs:106-108`, behavior at `crates/cli/src/pre_hook_cmd.rs:16` and `crates/cli/src/subcommands.rs:174-181`

**What**: Help says "Only check staged files instead of the entire workspace", but `--changed-only` only controls a preflight skip: when set, `has_staged_files` gates whether the hook runs at all. The user-configured hook command is invoked verbatim with no staged-paths substitution and no scoping — if it's `cargo fmt --check`, it still scans the whole workspace.

**Why it matters**: Users will read the help and expect path-level scoping. The actual semantics are "skip-when-no-staged" — a different contract. This is misleading documentation of a user-facing flag.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Help text for --changed-only on RunBeforeCommit accurately describes 'skip when nothing is staged' OR the implementation is extended to pass staged paths through
- [ ] #2 Help text for --changed-only on RunBeforePush is brought to the same standard
<!-- AC:END -->
