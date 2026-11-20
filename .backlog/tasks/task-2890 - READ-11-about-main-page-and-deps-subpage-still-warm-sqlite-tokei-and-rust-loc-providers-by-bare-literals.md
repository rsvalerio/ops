---
id: TASK-2890
title: 'READ-11: about main page and deps subpage still warm sqlite, tokei and rust-loc providers by bare literals'
status: Done
assignee: []
created_date: '2026-11-19 17:20'
updated_date: '2026-11-20 18:26'
labels:
  - code-review-rust
  - readability
dependencies: []
parent_task_id: 'TASK-2891'
modified_files:
  - extensions/about/src/lib.rs
  - extensions/about/src/deps.rs
  - crates/extension/src/provider_names.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/lib.rs:186`, `extensions/about/src/deps.rs:48`, `extensions/about/src/deps.rs:141`

**What**: `warm_providers(.., &["sqlite", "tokei", "rust-loc"], ..)` and `&["sqlite", CARGO_METADATA]` still name providers by string literals. The metadata key now lives in `ops_extension::provider_names`; the remaining keys have no shared constant with their producers.

**Why it matters**: Renaming one of those providers compiles cleanly while the about pages silently stop warming it.

**Origin**: discovered during TASK-2883 while fixing TASK-2877.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Every warm_providers call in ops-about names its providers via constants shared with the producers

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Added SQLITE, TOKEI, RUST_LOC, COVERAGE, CARGO_TOML to ops_extension::provider_names; producers' DATA_PROVIDER_NAME now alias them and every ops-about warm_providers call (plus the coverage get_or_provide) uses them.
<!-- SECTION:NOTES:END -->
