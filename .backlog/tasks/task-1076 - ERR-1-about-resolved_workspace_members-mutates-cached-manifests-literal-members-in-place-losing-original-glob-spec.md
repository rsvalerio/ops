---
id: TASK-1076
title: >-
  ERR-1: about resolved_workspace_members mutates cached manifest's literal
  members in place, losing original glob spec
status: Done
assignee: []
created_date: '2026-05-07 21:19'
updated_date: '2026-05-08 06:51'
labels:
  - code-review-rust
  - ERR
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/query.rs:188-190`

**What**: Lines 188-190 overwrite `ws.members` with the resolved list before wrapping in `Arc` and caching. Subsequent consumers cannot tell whether `[workspace].members = ["crates/*"]` was a glob or a literal list, and any code that re-runs glob expansion on the cached manifest no-ops.

**Why it matters**: Surprises a future consumer that wants the raw spec (linter, doc generator). Also a subtle correctness hazard with `ctx.refresh`: if the glob resolves to a different set, the cached Arc is replaced but anyone holding the previous Arc keeps the stale resolved list — interplays with TASK-1023 eviction.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Store resolved members alongside the literal spec (sibling field or wrapper struct), keeping ws.members immutable post-parse
- [x] #2 Update consumers to read the resolved view explicitly
- [x] #3 Test asserts that two Metadata objects derived from one CargoToml see consistent inputs and the cached manifest preserves ["crates/*"]
<!-- AC:END -->
