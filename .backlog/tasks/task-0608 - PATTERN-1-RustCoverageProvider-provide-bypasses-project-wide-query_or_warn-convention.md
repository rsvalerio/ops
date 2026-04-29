---
id: TASK-0608
title: >-
  PATTERN-1: RustCoverageProvider::provide bypasses project-wide query_or_warn
  convention
status: Triage
assignee: []
created_date: '2026-04-29 05:20'
labels:
  - code-review-rust
  - PATTERN
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/coverage_provider.rs:36`

**What**: All sister DuckDB calls in this crate (units, identity::metrics, deps_provider) route through query_or_warn with standard label/fallback shape; this site uses a bespoke `match { Err(e) => tracing::warn!(...); return Ok(default) }` block. Early-return-with-default diverges and makes the coverage-provider harder to compare with siblings.

**Why it matters**: Pure consistency, but the pattern is the spine of ERR-2/TASK-0376 family — every divergence is a new bug surface.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Total-coverage call rewritten to use query_or_warn with default ProjectCoverage (or a dedicated helper that preserves early-return semantics)
<!-- AC:END -->
