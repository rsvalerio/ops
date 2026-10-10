---
id: TASK-2541
title: 'READ-13: clock module docs carry a why-we-picked-chrono dependency essay'
status: To Do
assignee: []
created_date: '2026-10-10 15:37'
updated_date: '2026-10-10 21:12'
labels:
  - code-review-rust
  - readability
dependencies: []
parent_task_id: 'TASK-2613'
modified_files:
  - crates/backlog/src/clock.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:crates/backlog/src/clock.rs:clock'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/backlog/src/clock.rs:5-8` (module docs)

**What**: The module doc justifies the chrono dependency by narrating the build-graph research behind the choice: "That crate is already compiled into the `ops` binary through `duckdb -> arrow -> arrow-arith`, so depending on it directly costs no extra build time and no new supply-chain surface." The enduring, user-relevant properties are the ones already stated around it: dates come from chrono, never hand-rolled (TIME-1), and a pre-epoch clock is an error (ERR-6).

**Why it matters**: Per READ-13, "why we picked X over Y" essays are process artifacts — the duckdb/arrow chain is a snapshot of today's dependency graph that goes stale silently if a future refactor drops arrow-arith, while still claiming to be load-bearing. The dependency rationale belongs next to the Cargo.toml dependency entry (which already carries a one-line TIME-1 note) or in an ADR.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Module docs state the chrono delegation and error semantics without the build-graph dependency justification
<!-- AC:END -->
