---
id: TASK-1601
title: >-
  API-9: re-exported CoverageIngestor lacks #[non_exhaustive] despite being
  constructed only via load_coverage
status: Done
assignee:
  - TASK-1635
created_date: '2026-05-21 22:53'
updated_date: '2026-05-22 10:12'
labels:
  - code-review-rust
  - API
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/test-coverage/src/ingestor.rs:12` (re-exported at `extensions-rust/test-coverage/src/lib.rs:24`)

**What**: `CoverageIngestor` is a `pub` unit struct re-exported from `lib.rs`, so external consumers can construct it via `CoverageIngestor {}`. The sister `CoverageExtension` correctly carries `#[non_exhaustive]` with an API-9 / TASK-0922 rationale comment. The ingestor escaped the same treatment despite identical "construct only via the canonical entry point" semantics — `load_coverage` hardcodes the constructor.

**Why it matters**: Future fields on `CoverageIngestor` (timeout overrides, telemetry sink) become breaking changes for any external consumer that wrote the literal.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Annotate pub struct CoverageIngestor with #[non_exhaustive] and document rationale referencing the existing API-9 comment on CoverageExtension
- [ ] #2 Verify in-tree construction sites (lib.rs:73, tests) still compile (unit-struct literal allowed inside defining crate)
- [ ] #3 Decide whether CoverageIngestor needs to be pub from lib.rs at all; if no external consumer requires it, downgrade to pub(crate) and drop the re-export
<!-- AC:END -->
