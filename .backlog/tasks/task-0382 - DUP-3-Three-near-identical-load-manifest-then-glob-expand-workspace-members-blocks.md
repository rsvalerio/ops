---
id: TASK-0382
title: >-
  DUP-3: Three near-identical load manifest then glob-expand workspace members
  blocks
status: To Do
assignee:
  - TASK-0417
created_date: '2026-04-26 09:39'
updated_date: '2026-04-26 10:10'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/coverage_provider.rs:44` (also identity/mod.rs:55-58 and units.rs:33-37)

**What**: The pattern let mut manifest: CargoToml = serde_json::from_value(...); if let Some(ws) = &mut manifest.workspace { ws.members = resolve_member_globs(&ws.members, &cwd); } is repeated verbatim in three places.

**Why it matters**: Repeated mutation logic — any future change (e.g., honoring [workspace] exclude) must be made in three places.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extract shared helper resolved_workspace_members(manifest: &CargoToml, cwd: &Path) -> Vec<String>
- [ ] #2 All three providers use the helper
<!-- AC:END -->
