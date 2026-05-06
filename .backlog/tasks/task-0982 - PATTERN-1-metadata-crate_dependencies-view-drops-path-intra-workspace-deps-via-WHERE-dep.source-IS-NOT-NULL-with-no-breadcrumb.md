---
id: TASK-0982
title: >-
  PATTERN-1: metadata crate_dependencies view drops path/intra-workspace deps
  via WHERE dep.source IS NOT NULL with no breadcrumb
status: To Do
assignee:
  - TASK-1014
created_date: '2026-05-04 21:58'
updated_date: '2026-05-06 06:48'
labels:
  - code-review-rust
  - patterns
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/views.rs:24` (crate_dependencies_view_sql)

**What**: The `crate_dependencies` view filters every dependency where `dep.source IS NOT NULL`. Cargo metadata sets `source` to `null` for path dependencies (i.e. intra-workspace deps and locally-vendored crates). Those rows are silently dropped from the view, so any consumer of `query_crate_dep_counts` / `query_crate_deps` / the per-crate `dependency_count` reported by `RustIdentityProvider` and `RustDepsProvider` underreports workspace-internal coupling. There is no inline comment explaining the filter and no operator-visible signal that path deps were elided — about pages and the deps health gate render an artificially low dependency count.

**Why it matters**: The Rust about/units/deps stack uses dep counts as a project health signal. For workspaces that use path deps as the primary modularity tool (this very repo), the count can be off by 50%+ — yet the view's intent is opaque from the call sites. Either the filter belongs out (path deps are dependencies), or it should be a documented and named view (e.g. `external_crate_dependencies`) so consumers don't accidentally treat it as "all dependencies".

<!-- scan confidence: behaviour confirmed via SQL inspection; impact on consumers needs verification against per-crate dep_count outputs -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either the WHERE filter is removed (and downstream consumers reviewed for the count change) or the view is renamed to reflect that it lists external-source deps only, with a comment explaining the trade-off
- [ ] #2 Either path: a regression test asserts the chosen behaviour against a fixture workspace that includes one path dep and one registry dep
<!-- AC:END -->
