---
id: TASK-2322
title: 'Add a non-mutating fmt check for CI (ops fmt rewrites files)'
status: To Do
assignee: []
created_date: '2026-09-28 10:59'
updated_date: '2026-09-28 15:07'
labels:
  - ci
  - ops-alignment
dependencies: []
parent_task_id: 'TASK-2331'
modified_files: []
priority: high
ordinal: 1000
dedup_key: 'ops-align:ops-fmt-check'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**What**: the stack `fmt` is `cargo fmt --all`, which rewrites and exits 0, so `ops verify` in CI passes on unformatted code. ops's own ci.yml runs `cargo fmt --all` without `--check` too (CI never fails on formatting). Consumers must hand-define a check.

**Why it matters**: blocks CI running ops gates (forge TASK-0026, forge TASK-0024).

**Origin**: ops-alignment survey of forge, ops and ai, 2026-09-28.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A built-in check mode (e.g. `ops fmt --check` or a `fmt-check` command) that fails on unformatted code without writing
- [ ] #2 `verify` can run check-only in CI (flag, env or CI detection), including tw/eof
- [ ] #3 ops's own ci.yml fmt job actually fails on unformatted code
<!-- AC:END -->
