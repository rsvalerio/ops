---
id: TASK-2329
title: 'Move ops''s own ci.yml onto forge''s rust-ci and setup-ops'
status: Triage
assignee: []
created_date: '2026-09-28 10:59'
labels:
  - ci
  - ops-alignment
dependencies: []
modified_files: []
priority: low
ordinal: 1000
dedup_key: 'ops-align:ops-dogfood-ci'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**What**: ops does not consume forge rust-ci; its ~320-line ci.yml calls cargo directly with older action pins (checkout v6.1.0, setup-rust-toolchain v1.17.0, sccache v0.0.9, install-action v2.85.13) and duplicates fmt/check/clippy/test/deny jobs.

**Why it matters**: the tool that defines the gates should prove them in CI. Blocked on forge TASK-0023 and forge TASK-0026.

**Origin**: ops-alignment survey of forge, ops and ai, 2026-09-28.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 ops CI runs its gates via forge rust-ci (or setup-ops + `ops <gate>`), keeping only ops-specific jobs (miri, windows-backlog, msrv until ops msrv exists)
<!-- AC:END -->
