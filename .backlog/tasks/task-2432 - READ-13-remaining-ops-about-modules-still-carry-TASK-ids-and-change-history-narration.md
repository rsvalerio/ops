---
id: TASK-2432
title: 'READ-13: remaining ops-about modules still carry TASK ids and change-history narration'
status: To Do
assignee: []
created_date: '2026-10-04 15:51'
updated_date: '2026-10-10 14:31'
labels:
  - code-review-rust
  - READ
dependencies: []
parent_task_id: 'TASK-2439'
modified_files:
  - extensions/about/src/cards.rs
  - extensions/about/src/identity.rs
  - extensions/about/src/providers.rs
  - extensions/about/src/deps.rs
  - extensions/about/src/machine.rs
  - extensions/about/src/code.rs
  - extensions/about/src/loc.rs
  - extensions/about/src/coverage.rs
  - extensions/about/src/units.rs
  - extensions/about/src/test_support.rs
  - extensions/about/Cargo.toml
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/cards.rs`, `extensions/about/src/identity.rs`, `extensions/about/src/providers.rs`, `extensions/about/src/deps.rs`, `extensions/about/src/machine.rs`, `extensions/about/src/code.rs`, `extensions/about/src/loc.rs`, `extensions/about/src/coverage.rs`, `extensions/about/src/units.rs`, `extensions/about/src/test_support.rs`, `extensions/about/Cargo.toml`

**What**: TASK-2397 cleaned the files it listed (workspace.rs, manifest_cache.rs, manifest_io.rs, lib.rs, lru.rs). The other ops-about modules still hold about 57 `TASK-NNNN` references and "previously / used to / now" narration in doc and inline comments (cards.rs 11, identity.rs 9, providers.rs 9, deps.rs 7, machine.rs 7, code.rs 5, test_support.rs 4, coverage.rs 2, loc.rs 2, units.rs 1), plus a `DUP-3 / TASK-0985` comment on the `test-support` feature in Cargo.toml.

**Why it matters**: Same READ-13 class as TASK-2397: readers get history they cannot use, and it goes stale while looking authoritative.

**Origin**: discovered during TASK-2421 while fixing TASK-2397.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Docs and comments in the listed ops-about files state current behaviour and durable rationale only, with no TASK ids or past-behaviour narration
<!-- AC:END -->
