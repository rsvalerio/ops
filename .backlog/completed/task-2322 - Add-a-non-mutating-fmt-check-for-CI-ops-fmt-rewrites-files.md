---
id: TASK-2322
title: 'Add a non-mutating fmt check for CI (ops fmt rewrites files)'
status: Done
assignee: []
created_date: '2026-09-28 10:59'
updated_date: '2026-09-28 15:22'
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
- [x] #1 A built-in check mode (e.g. `ops fmt --check` or a `fmt-check` command) that fails on unformatted code without writing
- [x] #2 `verify` can run check-only in CI (flag, env or CI detection), including tw/eof
- [x] #3 ops's own ci.yml fmt job actually fails on unformatted code

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented in wave TASK-2331 (branch code-review/TASK-2331):
- AC1: `ops trailing-whitespace --check` / `ops end-of-file-fixer --check` report "would fix" without writing (same non-zero exit); Rust stack default `fmt-check` = `cargo fmt --all -- --check`.
- AC2: Rust stack default composite `verify-check` (the CI form of verify: fmt-check, trailing-whitespace-check, end-of-file-fixer-check, clippy, build, check-json, check-yaml, doc — all parallel, nothing writes). The two `*-check` twins are registered by the text-fixers extension (non-exclusive). Chosen as a separate command rather than env/CI detection: explicit, and needs no runtime rewriting of `fmt`.
- AC3: ci.yml fmt job now runs `cargo fmt --all -- --check`.
<!-- SECTION:NOTES:END -->
