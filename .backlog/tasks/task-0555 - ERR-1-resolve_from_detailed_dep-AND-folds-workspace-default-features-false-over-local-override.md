---
id: TASK-0555
title: >-
  ERR-1: resolve_from_detailed_dep AND-folds workspace default-features = false
  over local override
status: Triage
assignee: []
created_date: '2026-04-29 05:02'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/inheritance.rs:144`

**What**: The merge rule default_features: ws.default_features && local_default_features lets the workspace permanently disable defaults; combined with the missing default-features alias above, the effective resolved value is ws.default_features for nearly every dep. The optional merge ws.optional || local_optional has the symmetric issue: a workspace-required dep cannot be made optional locally.

**Why it matters**: Inheritance resolver disagrees with cargo actual precedence; downstream feature surface / optional dep displays are wrong whenever workspace and local disagree.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Document the cargo precedence rule the resolver implements (or fix to match cargo)
- [ ] #2 Add fixtures covering { workspace = true, default-features = true } against a workspace-disabled default and { workspace = true, optional = true } against a non-optional workspace dep
<!-- AC:END -->
