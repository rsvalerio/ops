---
id: TASK-2877
title: 'READ-11: generic about subpages warm the cargo metadata provider by a bare "metadata" literal'
status: Done
assignee: []
created_date: '2026-11-18 22:30'
updated_date: '2026-11-19 17:27'
labels:
  - code-review-rust
  - readability
dependencies: []
parent_task_id: 'TASK-2883'
modified_files:
  - extensions/about/src/units.rs
  - extensions/about/src/deps.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/units.rs:220`, `extensions/about/src/deps.rs:48`, `extensions/about/src/deps.rs:141`, `extensions/about/src/deps.rs:320`

**What**: `warm_providers(.., &["metadata"], ..)` names the metadata provider with a string literal. The producer's key is now the public `ops_metadata::DATA_PROVIDER_NAME`, and the Rust-side consumers read it through that constant, but the stack-agnostic `ops-about` crate cannot depend on the Rust-stack `ops-metadata` crate, so these warm calls still hardcode it.

**Why it matters**: A rename of the metadata provider compiles cleanly while the about subpages silently stop warming it, and the Rust providers then see a cache miss ("not warmed") and render empty units targets and duplicates.

**Origin**: discovered during TASK-2849 while fixing TASK-2742.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The warmed metadata provider name is a single named constant shared with the producer (e.g. owned by a crate both ops-about and ops-metadata already depend on) and every warm call site uses it

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
New ops_extension::provider_names::CARGO_METADATA; ops_metadata::DATA_PROVIDER_NAME aliases it and every metadata warm call in ops-about (units.rs, deps.rs x3) uses it. Remaining sqlite/tokei/rust-loc literals filed as TASK-2890.
<!-- SECTION:NOTES:END -->
