---
id: TASK-0381
title: >-
  FN-1: provide in identity provider mixes orchestration, parsing, and field
  assembly
status: To Do
assignee:
  - TASK-0417
created_date: '2026-04-26 09:38'
updated_date: '2026-04-26 10:10'
labels:
  - code-review-rust
  - structure
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/identity/mod.rs:50`

**What**: RustIdentityProvider::provide is ~40 lines but spans four abstraction levels: JSON deserialization, mutation of the manifest (ws.members = resolve_member_globs(...)), conditional field assembly, and serialization.

**Why it matters**: Future contributors are likely to duplicate the parse/mutate pattern in other providers (already happening in units.rs and coverage_provider.rs).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Introduce load_workspace_manifest(ctx) -> Result<CargoToml> that parses + resolves globs once
- [ ] #2 All three providers call the helper; remove the duplicated workspace-member mutation blocks
<!-- AC:END -->
