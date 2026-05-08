---
id: TASK-1042
title: >-
  PATTERN-1: about resolved_workspace_members never dedups, double-counts
  duplicate members in module_count and per-crate queries
status: Done
assignee: []
created_date: '2026-05-07 20:53'
updated_date: '2026-05-08 06:23'
labels:
  - code-review-rust
  - pattern
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/query.rs:222-325` (`resolved_workspace_members`)

**What**: After expanding `[workspace].members` globs the function only sorts and applies `exclude`:

```rust
resolved.retain(|m| !exclude.contains(m.as_str()));
resolved.sort();
resolved
```

There is no deduplication. A `Cargo.toml` that lists overlapping members — e.g.

```toml
members = ["crates/foo", "crates/*"]   # foo appears twice
# or
members = ["crates/*", "vendor/*"]     # if a symlink puts the same crate under both prefixes
```

— produces a `Vec<String>` with the same path repeated. Cargo itself dedupes this when building the workspace, so the manifest is legal and resolves cleanly, but the about pipeline:

1. `RustIdentityProvider::provide` reports `module_count = manifest.workspace.as_ref().map(|w| w.members.len())` (`identity/mod.rs:68`) — a duplicated member inflates the count visible on the about card.
2. `RustUnitsProvider::provide` (`units.rs:65-96`) iterates the (sorted) member list and emits one `ProjectUnit` per occurrence; the rendered units page shows the same crate twice.
3. `RustCoverageProvider::provide` (`coverage_provider.rs:74-91`) builds a `member_strs: Vec<&str>` and passes it to `query_crate_coverage` — duplicate keys silently re-query the same DB row but the resulting list still emits the unit twice.

The cached `Arc<CargoToml>` is re-used across all three providers (TASK-0558), so the duplication is locked in for the entire `ops about` invocation.

**Why it matters**: PATTERN-1 — the resolved list is presented as the workspace's authoritative member set; downstream consumers (display, counts, DB-keyed queries) trust it as a set, not a multiset. cargo's own behaviour is set-semantics, so silently diverging from cargo here lets the about UI claim the workspace has more crates than `cargo metadata` would report.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 resolved_workspace_members returns at most one entry per resolved member path (dedup post-glob, post-exclude)
- [x] #2 Regression test: members = ['crates/foo', 'crates/*'] with crates/foo present produces a single 'crates/foo' entry
<!-- AC:END -->
