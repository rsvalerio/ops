---
id: TASK-1505
title: >-
  READ-4: cargo-toml resolve_dep_from_workspace silently drops local overrides
  for version/path/git
status: To Do
assignee:
  - TASK-1643
created_date: '2026-05-18 18:04'
updated_date: '2026-05-25 16:08'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/inheritance.rs:155-228`

**What**: `extract_local_overrides` returns only `(features, optional, default_features)`. Any local `version`/`path`/`git`/`branch`/`tag`/`rev`/`target`/`package` field on a `workspace = true` dependency is silently discarded by `resolve_from_simple_dep` and `resolve_from_detailed_dep`. Cargo *does* reject these locally on a workspace-inherited dep, but the resolver does not match cargo's diagnostic behaviour — it silently strips them.

**Why it matters**: READ-4 / SEC-21-adjacent: the silent drop changes what downstream tooling sees (e.g. `about`/`deps` will not report a version conflict between the workspace and a malformed-but-readable member). The existing module doc-comment for `resolve_from_detailed_dep` documents merge precedence for the three handled fields but does not mention the dropped fields at all.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Document in resolve_dep_from_workspace (or on extract_local_overrides) which local fields are intentionally ignored and why (matching cargo's 'member cannot override workspace dep source' rule)
- [ ] #2 Optionally: emit a tracing::debug! when a local override on version/git/path is observed and dropped, so operators can see the silent transform
<!-- AC:END -->
