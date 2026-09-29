---
id: TASK-2329
title: 'Move ops''s own ci.yml onto forge''s rust-ci and setup-ops'
status: In Progress
assignee: []
created_date: '2026-09-28 10:59'
updated_date: '2026-09-29 17:14'
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
- [x] #1 ops CI runs its gates via forge rust-ci (or setup-ops + `ops <gate>`), keeping only ops-specific jobs (miri, windows-backlog, msrv until ops msrv exists)

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
ci.yml now calls forge rust-ci@v1 with engine: ops (verify-check, deps --check, sec, run-msrv) and keeps only ops-specific jobs: clippy-default, test (next under NEXTEST_PROFILE=ci + skip surfacing + next-ignored + test-doc), windows-backlog, miri, workflow-guard. The kept jobs set up via forge actions/setup-rust / setup-tools (ops at forge's mise.toml pin, via setup-ops) instead of building this checkout's ops. docs/releasing.md check table updated. Before merge: the main-protection ruleset requires 'ops verify' and 'ops qa', which no longer exist; replace them with 'rust-ci / ops verify-check' and 'ops test'. Close once the PR's CI is green.

ops side in rsvalerio/ops#81.

<!-- SECTION:NOTES:END -->
