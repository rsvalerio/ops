---
id: TASK-2327
title: 'Add an msrv command that checks the workspace against rust-version'
status: To Do
assignee: []
created_date: '2026-09-28 10:59'
updated_date: '2026-09-28 15:07'
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
- [ ] #1 `ops msrv` reads rust-version, runs check with that toolchain and verifies clippy.toml msrv matches
<!-- AC:END -->
