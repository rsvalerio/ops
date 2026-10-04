---
id: TASK-2361
title: 'READ-13: ops-theme docs narrate change history instead of the end state'
status: Triage
assignee: []
created_date: '2026-10-04 14:10'
labels:
  - code-review-rust
  - READ
dependencies: []
modified_files:
  - crates/theme/src/configurable.rs
  - crates/theme/src/configurable/boxed.rs
  - crates/theme/src/configurable/config_access.rs
  - crates/theme/src/style.rs
  - crates/theme/src/style/sgr.rs
  - crates/theme/src/render.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:crates/theme/src:module-docs'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/theme/src/configurable.rs:6-9,39-43`, `crates/theme/src/configurable/boxed.rs:3-5,263-266`, `crates/theme/src/configurable/config_access.rs:3-7`, `crates/theme/src/style.rs:14-15`, `crates/theme/src/style/sgr.rs:34-47,114-120`, `crates/theme/src/render.rs:50-56`

**What**: `//!` and `///` blocks describe how the code got here rather than what it does now: "The module previously mixed it with ... display_width" (configurable.rs), "was measured afresh on every rendered row ... before being hoisted" (configurable.rs `icon_column_width`), "had grown past the module red flag" (boxed.rs, config_access.rs), "previously pushed the closing corner past `columns`" (boxed.rs `build_horizontal_border`), "preserve the previous module-level API" (style.rs), "DUP-3 / TASK-1188 routed ... the distinction TASK-1188's single resolver had erased" and "The eager convenience form ... was removed with its last caller (TASK-2256)" (sgr.rs), "used to be covered by a local re-implementation in the test module" (render.rs). Doc prose is also threaded with rule/TASK tags.

<!-- scan confidence: candidates to inspect -->

**Why it matters**: Doc comments that narrate migrations or earlier designs mean nothing to an API reader and go stale. The text belongs in commit messages or PR descriptions. Keep the enduring invariants (width policy, truncation policy, gate semantics) and drop the history.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Doc comments state current behaviour and invariants only; no 'previously', 'used to', 'was removed', or migration narration remains
- [ ] #2 Enduring policies (width measurement, truncation, colour gating) stay documented
<!-- AC:END -->
