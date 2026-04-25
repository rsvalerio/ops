---
id: TASK-0293
title: >-
  ARCH-1: crates/runner/src/command/exec.rs is ~760 lines mixing build, exec,
  env-guard, and secret detection
status: Done
assignee:
  - TASK-0298
created_date: '2026-04-23 16:54'
updated_date: '2026-04-23 17:21'
labels:
  - rust-code-review
  - architecture
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/exec.rs:1-760`

**What**: Single module combines at least four distinct concerns: command building / cwd resolution, async process execution with timeouts, environment validation, and secret-pattern heuristics (looks_like_jwt / looks_like_aws_key / etc.). 39 functions, 760 lines.

**Why it matters**: ARCH-1 flags modules >500 lines with unrelated concerns. Secret-detection is security-critical and would benefit from being independently testable in a `secret_patterns` submodule; env and exec logic are orthogonal and evolve at different cadences.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Secret-pattern detection extracted into a dedicated submodule (e.g. runner::secret_patterns) with its own unit tests
- [x] #2 Command building / cwd resolution split from async exec into separate submodules or files
- [x] #3 exec.rs (or its split successors) each under ~400 lines and pass cargo build + cargo test
<!-- AC:END -->
