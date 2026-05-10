---
id: TASK-0302
title: >-
  SEC-25: detect_workspace_escape canonicalizes then spawns — symlink TOCTOU
  under Deny policy
status: Done
assignee:
  - TASK-0323
created_date: '2026-04-24 08:52'
updated_date: '2026-04-25 12:23'
labels:
  - rust-code-review
  - security
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/build.rs:64-81`

**What**: The workspace-escape guard canonicalizes and validates the cwd, then later spawns `Command::new(program).current_dir(cwd)`. A symlink swap between the check and `spawn` can bypass the guard while `CwdEscapePolicy::Deny` advertises fail-closed behavior.

**Why it matters**: Deny is the security-critical mode; users rely on it to prevent execution outside the project boundary, but a race window still permits escape.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Under Deny, either open the directory via a handle and pass the fd to the child, OR the residual TOCTOU window is explicitly documented on the policy variant
- [x] #2 Test added that simulates cwd swap between check and spawn (best-effort, may be gated by OS)
<!-- AC:END -->
