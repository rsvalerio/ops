---
id: TASK-2227
title: 'PATTERN-1: about-node hardcodes module_count = None, so the ''packages'' card row never renders for the monorepos its units provider does enumerate'
status: Done
assignee: []
created_date: '2026-09-08 07:22'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-rust
  - correctness
dependencies: []
parent_task_id: 'TASK-2239'
modified_files:
  - extensions-node/about/src/lib.rs
priority: medium
ordinal: 133000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-node/about/src/lib.rs:98`

**What**: `NodeIdentityProvider::provide` sets

```rust
m.module_label = "packages";
m.module_count = None;
```

unconditionally. `crates/core/src/project_identity/card.rs:81` renders the
modules row as `label: id.module_label, value: id.module_count.map(...)`, so a
`None` count means the row is dropped entirely and the `module_label` string
set one line above is dead — it can never appear.

Meanwhile the sibling provider registered by the very same extension,
`units::NodeUnitsProvider` (`extensions-node/about/src/units.rs:38`), fully
resolves npm/yarn `workspaces` and `pnpm-workspace.yaml` members, so `ops
about` on a pnpm monorepo lists N packages in the units table while the
identity card shows no packages count at all. Both providers already read the
same `package.json` through `ops_about::manifest_cache`, so the count is
available with no extra IO.

Every other stack computes it: `extensions-rust/about/src/identity/mod.rs:75`,
`extensions-java/about/src/maven/mod.rs:28`, `extensions-java/about/src/gradle/mod.rs:40`,
`extensions-go/about/src/lib.rs:85`.

**Why it matters**: the About card silently under-reports the shape of the
project, and the discrepancy points the reader at the wrong conclusion — a
12-package pnpm workspace looks like a single-package repo on the card and a
12-row table below it.

**Twins** (same class, other stacks — cross-reference, do not merge):
TASK-2203 (about-python, identical hardcoded `None`) and TASK-2178 (about-go,
count diverges from the units list rather than being absent).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 module_count reflects the workspace member count the units provider resolves for npm/yarn workspaces and pnpm-workspace.yaml
- [x] #2 a single-package project (no workspaces declaration) still yields module_count = None so the row stays hidden
- [x] #3 the count is derived from the cached package.json read, adding no second manifest IO
- [x] #4 a test asserts the identity card's module_count equals the length of NodeUnitsProvider's output for the same fixture
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Fixed: units.rs exposes resolved_members (the shared resolve_member_globs call collect_units already used) and the identity provider sets module_count = Some(len) when the workspace resolves members, None for single-package projects. The root package.json read goes through the shared manifest_cache entry (AC #3 — no second manifest IO; only the same dir-listing probes the units provider performs). Tests: parse_minimal_package_json pins the single-package None (AC #2); workspace_module_count_equals_the_units_provider_length asserts identity module_count == NodeUnitsProvider units len on the same fixture for both npm workspaces and pnpm-workspace.yaml sources, including a non-resolving member dir (AC #1/#4). cargo test -p ops-about-node: 113 passed; clippy pedantic clean.
<!-- SECTION:NOTES:END -->
