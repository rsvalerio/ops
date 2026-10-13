---
id: TASK-2633
title: 'READ-13: about crate lib.rs, units.rs and manifest_cache.rs still carry rule-ID/TASK provenance tags'
status: Triage
assignee: []
created_date: '2026-10-11 12:34'
labels:
  - code-review-rust
  - readability
dependencies: []
modified_files:
  - extensions-rust/about/src/lib.rs
  - extensions-rust/about/src/units.rs
  - extensions-rust/about/src/manifest_cache.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/lib.rs`, `extensions-rust/about/src/units.rs`, `extensions-rust/about/src/manifest_cache.rs`

**What**: The wave-74 strip (TASK-2629) cleaned the six files its scope named, but the same crate's remaining files still carry RULE-ID / TASK-XXXX provenance-tag prefixes on module, item and test docs — e.g. `lib.rs` (`API-14 / TASK-2163:`, `DUP-3 / TASK-1814:`, `SEC-14 / TASK-1246 member-path`), `units.rs` (~20 prefix sites: `ERR-2 / TASK-0376:`, `SEC-14 / TASK-1246 AC #1:`, `PERF-3 / TASK-1569:`, ...), and `manifest_cache.rs` (`CONC-7 / TASK-1163: concurrency contract` heading and ~10 more). Note: the `CONC-7` contract is cross-referenced by name from already-cleaned files (`manifest.rs`, `workspace_root_cache.rs`, `coverage_provider.rs`), so if the heading is renamed those cross-references must be updated in the same pass.

**Why it matters**: READ-13 — docs must describe the end state; tag prefixes are process artifacts that rot while looking authoritative. The crate is now inconsistently cleaned, which makes the residue look deliberately kept rather than not-yet-swept.

**Origin**: discovered during TASK-2632 (code-review-plan-wave74) while fixing TASK-2629; those files were outside the member's modified-file scope and outside the wave's declared file scope.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every tag-prefix site in the three files states the end state in present-tense prose with no RULE/TASK prefix, or is a deliberate cross-reference to a named contract (CONC-7 cross-references updated in the same pass if the contract heading is renamed)
<!-- AC:END -->
