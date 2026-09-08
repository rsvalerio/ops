---
id: TASK-2097
title: 'API-14: ops-core crate root has no crate-level documentation'
status: To Do
assignee:
  - TASK-2247
created_date: '2026-09-07 22:59'
updated_date: '2026-09-08 10:59'
labels:
  - code-review-rust
  - api
dependencies: []
modified_files:
  - crates/core/src/lib.rs
priority: low
ordinal: 22000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/lib.rs:1-19`

**What**: The crate root opens with a `#![cfg_attr(test, allow(...))]` attribute and then a bare `pub mod` list — there is no `//!` crate-level doc anywhere in the file. Every one of the fourteen public modules the list exposes carries its own module-level `//!` docs (verified: config, expand, output, paths, project_identity, report, serde_defaults, stack, style, subprocess, table, text, ui), so only the crate root is undocumented.

**Why it matters**: API-14 — `cargo doc` renders an empty summary page for `ops_core` itself; a reader landing on the crate (docs.rs-style browsing, `cargo doc --open`, IDE hover on `ops_core::`) gets no statement of what the crate is, how its layers fit together (config loading/merging, stack detection, variable expansion, subprocess running, output/UI), or which module is the intended entry surface. The sibling reviews already filed API-14 for missing doc summaries on public items in `ops-about` (TASK-2071); this is the same gap at crate granularity for `ops-core`. One `//!` block (3-6 lines plus a module map) closes it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 lib.rs starts with a crate-level doc comment stating what ops-core is and the role of its public modules (a short module map is enough); cargo doc renders a non-empty crate summary page
<!-- AC:END -->
