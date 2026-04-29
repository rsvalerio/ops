---
id: TASK-0541
title: >-
  ERR-1: Config::validate skips composite specs; unknown refs and cycles slip
  past load-time
status: Triage
assignee: []
created_date: '2026-04-29 04:58'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/mod.rs:66`

**What**: `Config::validate` only invokes `exec.validate(name)` for `CommandSpec::Exec`. Composites are never validated at load time — typos in `commands = ["buidl"]`, self-cycles, and depth violations only surface when the user invokes the affected command.

**Why it matters**: Strict-merge and exec validation set a precedent of failing at load time; composite typos break that contract and produce confusing run-time errors.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 validate verifies every composite's referenced names resolve (against config + stack defaults + a registered extension list passed in) and detects cycles, without standing up a CommandRunner
- [ ] #2 Tests cover unknown-ref, self-cycle, and depth violations at the validate boundary
<!-- AC:END -->
