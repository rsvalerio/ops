---
id: TASK-0203
title: >-
  API-5: CommandRunner::expand_to_leaves returns Option<Vec<CommandId>> losing
  the reason for failure
status: To Do
assignee: []
created_date: '2026-04-22 21:26'
updated_date: '2026-04-23 06:45'
labels:
  - rust-code-review
  - ERR
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/mod.rs:263-307` (expand_to_leaves / expand_inner).

**What**: `expand_to_leaves` returns `Option<Vec<CommandId>>` where `None` conflates three distinct failure modes: (1) unknown command id, (2) cycle detected (visited insert failed), (3) max depth exceeded. Callers (CLI `run_cmd.rs`, `run_command_cli`, `run_command_raw`, etc.) reconstruct the error message as `"unknown command: {}"` for every `None` case — which is actively wrong when the real cause was a cycle or depth-limit. The `tracing::warn!` for depth-exceeded logs the true cause but the user sees "unknown command".

**Why it matters**: ERR-10 + API-5 (lossy Option where Result<T, E> is warranted). Fix: introduce `enum ExpandError { Unknown(String), Cycle(String), DepthExceeded { id: String, depth: usize } }` and return `Result<Vec<CommandId>, ExpandError>`. Callers can then emit accurate error messages. Related to TASK-0130 (ERR-10 on resolve_exec_leaf) — same anti-pattern, different function.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 expand_to_leaves returns Result<Vec<CommandId>, ExpandError> distinguishing unknown/cycle/depth
- [ ] #2 run_cmd callers surface the specific cause
<!-- AC:END -->
