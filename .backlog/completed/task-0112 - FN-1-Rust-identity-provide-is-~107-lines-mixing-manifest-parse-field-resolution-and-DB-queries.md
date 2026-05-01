---
id: TASK-0112
title: >-
  FN-1: Rust identity provide() is ~107 lines mixing manifest parse, field
  resolution, and DB queries
status: Done
assignee: []
created_date: '2026-04-19 18:36'
updated_date: '2026-04-19 19:43'
labels:
  - rust-code-review
  - functions
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/identity.rs:61-167`

**What**: `<RustProjectIdentityProvider as DataProvider>::provide` spans lines 61–167 and orchestrates: Cargo.toml parse, workspace-member glob expansion, per-field inheritance resolution (7 fields), authors fallback, DuckDB LOC / dependency / coverage queries, and `ProjectIdentity` assembly.

**Why it matters**: Exceeds FN-1 50-line guidance and mixes multiple abstraction levels in one function; each concern is separately testable.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Behavior and existing tests unchanged
- [x] #2 Extract at least two helpers (resolve_identity_fields, query_identity_metrics) so provide becomes an orchestrator under ~50 lines
- [x] #3 Behavior and existing tests unchanged
<!-- AC:END -->
