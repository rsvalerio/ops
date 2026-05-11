---
id: TASK-1274
title: 'API-1: run-before-push silently discards --changed-only flag'
status: To Do
assignee:
  - TASK-1305
created_date: '2026-05-11 15:24'
updated_date: '2026-05-11 16:48'
labels:
  - code-review-rust
  - api
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/subcommands.rs:195-207`

**What**: `run_before_push` takes `_changed_only: bool` but hard-codes `false` when calling `run_hook_dispatch(..., false)`. The CLI surface (`args.rs:117-123`) advertises `--changed-only` on `run-before-push`, so the flag parses successfully and is then thrown away by the underscore-prefixed parameter.

**Why it matters**: A user passing `ops run-before-push --changed-only` gets the all-files run with no warning — documented behaviour diverges from actual behaviour. This is a correctness/UX bug, not a stylistic nit. Either the `_` prefix hides an unfinished feature, or the flag should be removed from the args struct.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either forward changed_only through to run_hook_dispatch (matching run_before_commit) and add a parse->execute integration test asserting the flag propagates
- [ ] #2 Or drop RunBeforePush.changed_only from args.rs and update the help text, with a test that the flag is rejected by clap
- [ ] #3 Underscore-prefixed unused parameter no longer hides the discrepancy
<!-- AC:END -->
