---
id: TASK-2157
title: 'PATTERN-1: manifest cache freshness ignores the glob-expanded member set it caches'
status: Done
assignee: []
created_date: '2026-09-08 07:04'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-rust
  - pattern
dependencies: []
parent_task_id: 'TASK-2239'
modified_files:
  - extensions-rust/about/src/manifest_cache.rs
  - extensions-rust/about/src/manifest.rs
priority: low
ordinal: 70000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/manifest_cache.rs:17-18`, `extensions-rust/about/src/manifest.rs:66-82`, `extensions-rust/about/src/manifest.rs:148-160`

**What**: The typed-manifest cache's stated invalidation contract is "the `<root>/Cargo.toml` mtime+len pair is re-stat'ed on every probe; a mismatch reparses" (`manifest_cache.rs:17-18`). But the cached value is a `LoadedManifest`, which carries two derived views that are *not* functions of that file's bytes:

- `resolved_members` — computed in `LoadedManifest::new` by `resolved_workspace_members`, which `read_dir`s `workspace_root/<prefix>` for every `prefix/*` glob;
- `canonical_member_manifests` — an `Arc<OnceLock<HashMap<..>>>` built by `fs::canonicalize`ing each member's `Cargo.toml`.

For the overwhelmingly common `members = ["crates/*"]` shape, creating or deleting a member directory changes neither the root manifest's mtime nor its length, so the freshness key matches and the cache serves the pre-change member list indefinitely. `ctx.refresh` is the only escape hatch. The behaviour is test-pinned (`manifest.rs:317-323`, `resolved_workspace_members_are_amortised_via_typed_manifest_cache`, which asserts the cached view does *not* pick up a member-directory change) but the module's own "Invalidation" bullet does not mention it.

**Why it matters**: The module docs repeatedly name daemon / language-server / CI-worker hosts as a target shape (`manifest_cache.rs:60-72`, `coverage_provider.rs:45-48`, `workspace_root_cache.rs:39-42`). In any such host, `cargo new crates/foo` is invisible to `ops about` for the process lifetime: `module_count` (identity), the `ProjectUnit` list (units) and the per-crate coverage table all silently diverge from `cargo metadata`, with no warn and no way for a caller to tell a stale answer from a fresh one. In the single-shot CLI the process boundary hides it, which is exactly why it would ship unnoticed. Either the freshness key must cover the member set (e.g. also stat the glob prefix directories, whose mtime *does* change on child creation/removal) or the contract must say out loud that a cached `LoadedManifest`'s member list is frozen until `ctx.refresh`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A decision is recorded: either the freshness key is extended to cover the resolved member set, or the cache contract explicitly documents that the member list and canonical-manifest map are frozen for the entry's lifetime
- [x] #2 If the key is extended, adding or removing a directory under a `prefix/*` glob causes the next `load_workspace_manifest` to re-resolve, and a test pins that (the existing `resolved_workspace_members_are_amortised_via_typed_manifest_cache` test is updated to assert the amortisation it actually still guarantees)
- [x] #3 If the contract is documented instead, the `# Cache contract` **Invalidation** bullet in `manifest_cache.rs` names the two derived views and states that a host which can outlive a member-set change must call `ctx.refresh`
- [x] #4 The reviewer rule at `manifest_cache.rs:74-76` ("do not add a daemon caller without first making the migration above") is extended to cover this staleness, so a future daemon caller cannot land without addressing it
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Decision (TASK-2157 AC #1): contract documented, key NOT extended.

Rationale: extending the freshness key to the glob prefixes would (a) restructure the probe API — the key is computed before the cache lookup, but the prefixes are only knowable from the already-cached manifest; (b) break the documented one-stat hot-path budget (PERF-1 / TASK-2028); and (c) still be incomplete — stat-ing the prefix directory catches directory add/remove but not an existing directory gaining a Cargo.toml, so the contract would over-promise a freshness it cannot deliver. No daemon caller exists (documented reviewer rule), and ctx.refresh is the escape hatch.

Applied: manifest_cache.rs gains a **Member-set freeze** invalidation bullet (AC #3) naming resolved_members and canonical_member_manifests, and the daemon reviewer rule is extended to require key coverage or ctx.refresh-on-workspace-events before any daemon caller may rely on ops about member data (AC #4). The pinned test resolved_workspace_members_are_amortised_via_typed_manifest_cache now cross-references the documented contract. AC #2 is conditional on extending the key, which this decision declines — vacuously satisfied.
<!-- SECTION:NOTES:END -->
