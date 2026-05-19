---
id: TASK-1497
title: >-
  DUP-1: cargo-toml find_workspace_root_*_with_depth share ~50 lines of walk
  scaffolding
status: To Do
assignee:
  - TASK-1573
created_date: '2026-05-18 18:03'
updated_date: '2026-05-19 16:45'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/lib.rs:374-449` and `:478-566`

**What**: `find_workspace_root_with_depth` and `find_workspace_root_strict_with_depth` duplicate the canonicalize-or-route-NotFound preamble (15 lines), the depth loop (`for _ in 0..max_depth`), the `try_exists`+`tracing::warn` skip-and-continue block, the `match current.parent()` walk-step, and the trailing `first_cargo_toml` fallback + final `NotFound` return. Only the per-candidate decision (`manifest_declares_workspace` vs canonicalize-then-prefix-check) differs.

**Why it matters**: Any future fix to walk semantics (a new `tracing` field, a different `try_exists` policy, depth-counter telemetry, SEC-25 follow-ups) has to be applied in two places; the existing comments already differ in TASK references (`TASK-0988` vs `TASK-1204`), proving the drift hazard is real, not theoretical.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Walk loop, canonicalize preamble, and NotFound fallback live in one function/closure parameterised by the per-candidate check
- [ ] #2 Both find_workspace_root_with_depth and find_workspace_root_strict_with_depth delegate to the shared core; tests pass unchanged
- [ ] #3 No behavioural divergence between lenient and strict variants beyond the per-candidate check
<!-- AC:END -->
