---
id: TASK-2323
title: 'Support --locked across the Rust stack defaults'
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
priority: medium
ordinal: 1000
dedup_key: 'ops-align:ops-locked'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**What**: no stack command passes `--locked` (only clippy-findings does). event0 re-declares 9 built-ins (build, check, clippy, doc, test, test-ignored, next, next-ignored, test-doc) solely to add it.

**Why it matters**: CI must not resolve a different lockfile than the one committed; the fix belongs in the defaults, not in every repo.

**Origin**: ops-alignment survey of forge, ops and ai, 2026-09-28.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Rust stack cargo commands run `--locked` by default or via one switch (flag/env/config)
- [x] #2 event0 can drop its re-declared built-ins (follow-up noted there)

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented in wave TASK-2331 (branch code-review/TASK-2331):
- AC1: one switch, `[cargo] locked = true` in .ops.toml or `OPS__CARGO__LOCKED=true` via the env overlay. Applied at config load after [extend] (crates/core/src/config/locked.rs): adds `--locked` before any `--` to every `cargo` exec command whose subcommand is build/check/clippy/doc/test/nextest/run/bench (stack defaults materialized, config commands in place), skipping ones already passing --locked/--frozen. Not the default because `--locked` refuses to create a missing Cargo.lock (verified), which would break lockless workspaces.
- AC2 (~, substituted): the ops side is done — event0 can replace its 9 re-declared built-ins with `[cargo] locked = true`. The "follow-up noted there" half is a write to the separate event0 repo, outside this ops code-review run; left for the user: in event0, drop the re-declared build/check/clippy/doc/test/test-ignored/next/next-ignored/test-doc and add `[cargo]\nlocked = true` once on an ops release with TASK-2323.
<!-- SECTION:NOTES:END -->
