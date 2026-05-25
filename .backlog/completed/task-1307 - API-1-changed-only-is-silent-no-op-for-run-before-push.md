---
id: TASK-1307
title: 'API-1: --changed-only is silent no-op for run-before-push'
status: Done
assignee:
  - TASK-1382
created_date: '2026-05-11 19:57'
updated_date: '2026-05-12 22:59'
labels:
  - code-review-rust
  - API
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/pre_hook_cmd.rs:27`, plumbing at `crates/cli/src/subcommands.rs:211` and `crates/cli/src/args.rs:115-120`

**What**: `RunBeforePush::changed_only` is parsed, plumbed through `run_hook_action` → `run_hook_dispatch(.., run_preflight=true)`, but `PUSH_OPS.preflight = None` (pre_hook_cmd.rs:27), so `if run_preflight { if let Some((predicate, skip_msg)) = hook.preflight { ... } }` short-circuits and the flag has no observable effect. The recent commit message for TASK-1274 claims this was fixed ("changed_only is forwarded for both hooks (was silently dropped for run-before-push)"), but forwarding it to a `None` predicate is functionally identical to dropping it.

**Why it matters**: The documented behavior ("Only check changed files instead of the entire workspace", args.rs:118) is unimplemented. `ops run-before-push --changed-only` parses successfully and silently does the same thing as without the flag — exactly the class of regression TASK-1274 was meant to close.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 ops run-before-push --changed-only either implements a meaningful filter OR is removed/rejected with a clear error — not a silent no-op
- [ ] #2 Test pins that changed_only=true vs changed_only=false produces user-observable behavior difference for run-before-push
<!-- AC:END -->
