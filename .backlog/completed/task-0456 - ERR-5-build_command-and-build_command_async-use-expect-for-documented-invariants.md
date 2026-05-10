---
id: TASK-0456
title: >-
  ERR-5: build_command and build_command_async use expect() for documented
  invariants
status: Done
assignee:
  - TASK-0537
created_date: '2026-04-28 05:44'
updated_date: '2026-04-28 17:04'
labels:
  - code-review-rust
  - ERR
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/build.rs:179` and `:200`

**What**: `build_command` does `.expect("WarnAndAllow policy never returns Err")`. `build_command_async` does `.expect("build_command panicked on blocking pool")` on the spawn_blocking JoinError. Both panic on conditions documented as "cannot happen" but not enforced by the type system.

**Why it matters**: A future maintainer adding an early-return error path in resolve_spec_cwd would panic every interactive `cargo ops` run. The JoinError case turns a panic-propagation/runtime-shutdown into a CLI panic instead of a graceful step-failure.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 build_command rewritten so the no-error guarantee is structural (e.g. private helper sharing logic with build_command_with returning Command directly for WarnAndAllow inputs)
- [x] #2 build_command_async either propagates the JoinError up (returning Result) or downgrades to tracing::error! plus a synthesized failing Command so a panicking blocking task does not abort the runner
<!-- AC:END -->
