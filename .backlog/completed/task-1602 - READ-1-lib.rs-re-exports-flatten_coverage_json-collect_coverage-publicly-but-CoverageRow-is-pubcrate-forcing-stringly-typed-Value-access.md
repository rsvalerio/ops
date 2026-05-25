---
id: TASK-1602
title: >-
  READ-1: lib.rs re-exports flatten_coverage_json/collect_coverage publicly but
  CoverageRow is pub(crate), forcing stringly-typed Value access
status: Done
assignee:
  - TASK-1635
created_date: '2026-05-21 22:53'
updated_date: '2026-05-22 10:12'
labels:
  - code-review-rust
  - READ
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Files**:
- `extensions-rust/test-coverage/src/lib.rs:25` (`pub use parse::{collect_coverage, flatten_coverage_json}`)
- `extensions-rust/test-coverage/src/parse.rs:21` (`pub(crate) struct CoverageRow`)

**What**: The two public functions return `serde_json::Value` because their natural typed return — `Vec<CoverageRow>` — is crate-private. External callers must indexing-access fields by string name, defeating the DUP-3 single-source-of-truth that `CoverageRow` was designed to enforce (TASK-1555).

**Why it matters**: Encourages downstream code that re-implements field-name strings, which is the schema-drift hazard TASK-1555 eliminated *inside* the crate. The public API leaks the hazard back to consumers.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either promote CoverageRow to pub (with #[non_exhaustive]) and change flatten_coverage_json return type to Result<Vec<CoverageRow>, _>, or demote flatten_coverage_json/collect_coverage to pub(crate)
- [ ] #2 Update provider::query_coverage_files and ingestor::collect to consume the typed form on the in-crate path
- [ ] #3 Document the decision in lib.rs near the re-export
<!-- AC:END -->
