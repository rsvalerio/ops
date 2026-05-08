---
id: TASK-1200
title: >-
  API: ToolStatus collapses probe failure into NotInstalled, driving spurious
  reinstalls when rustup is wedged
status: To Do
assignee:
  - TASK-1269
created_date: '2026-05-08 08:14'
updated_date: '2026-05-08 13:19'
labels:
  - code-review-rust
  - api
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/tools/src/probe.rs:35-63,521-565`; `extensions-rust/tools/src/lib.rs:65-80`

**What**: run_probe_with_timeout warns and returns None for timeout / spawn / unrecognised-error variants; every caller maps that into "not installed". ToolStatus has only Installed / NotInstalled. A timed-out `rustup show active-toolchain` makes every tool look uninstalled, and downstream install_tool will then attempt to reinstall a perfectly working toolchain — turning a transient probe failure into a real mutation.

**Why it matters**: The type system promises two states but the function actually has three. Until the third variant is restored, check_tool_status is a contract that lies.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 ToolStatus gains an explicit ProbeFailed (or equivalent) variant; run_probe_with_timeout callers route timeout / IO-error returns through it instead of NotInstalled.
- [ ] #2 install_tool treats ProbeFailed as 'skip with warn'; a regression test pins that a timed-out probe does NOT trigger a reinstall.
<!-- AC:END -->
