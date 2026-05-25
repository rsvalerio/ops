---
id: TASK-1585
title: 'READ-1: two timeout.rs modules in the tools crate invite import confusion'
status: Done
assignee:
  - TASK-1638
created_date: '2026-05-21 22:45'
updated_date: '2026-05-22 13:16'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/tools/src/timeout.rs` and `extensions-rust/tools/src/probe/timeout.rs`

**What**: The crate has two unrelated modules named `timeout`: `crate::timeout` exports `run_with_timeout` (install-side bounded child wait) and `crate::probe::timeout` exports `ProbeOutcome` and `run_probe_with_timeout` (probe-side bounded child wait wrapping ops_core::subprocess::run_with_timeout). Both wrap subprocess timeouts but with different APIs and error contracts; nothing in the module path disambiguates which `run_with_timeout` is in play, and tests already import both via `use super::*` / `use crate::probe::...`.

**Why it matters**: READ-1 — names should disambiguate concerns. A future contributor reading `run_with_timeout(...)` cannot tell whether the install bounded wait or the probe wrapper is intended without checking the import. Rename one (e.g. `crate::timeout` → `crate::install_timeout` or fold into `install.rs`, and/or `crate::probe::timeout` → `crate::probe::run`) so the names carry their domain.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 the two modules no longer share the name 'timeout', or one is inlined into its sole consumer (install.rs / probe/mod.rs)
- [x] #2 no public symbols are renamed without a deprecation alias
<!-- AC:END -->
