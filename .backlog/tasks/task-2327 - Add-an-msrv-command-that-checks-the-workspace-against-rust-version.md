---
id: TASK-2327
title: 'Add an msrv command that checks the workspace against rust-version'
status: Done
assignee: []
created_date: '2026-09-28 10:59'
updated_date: '2026-09-28 15:24'
labels:
  - ci
  - ops-alignment
dependencies: []
parent_task_id: 'TASK-2334'
modified_files: []
priority: low
ordinal: 1000
dedup_key: 'ops-align:ops-msrv'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**What**: ops's ci.yml hand-rolls an MSRV job; forge has none; the clippy-pedantic skill requires clippy `msrv` == `rust-version`.

**Why it matters**: one implementation for ops's CI and forge TASK-0029.

**Origin**: ops-alignment survey of forge, ops and ai, 2026-09-28.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 `ops msrv` reads rust-version, runs check with that toolchain and verifies clippy.toml msrv matches

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Landed in fceb23f2/01237b74/c886bf1d on code-review/run-20260928: `ops msrv [--install]` checks clippy.toml msrv == rust-version and runs `rustup run <v> cargo check --workspace --all-features --all-targets`; CI MSRV job now runs it.
<!-- SECTION:NOTES:END -->
