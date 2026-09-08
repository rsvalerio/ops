---
id: TASK-2157
title: >-
  PATTERN-1: manifest cache freshness ignores the glob-expanded member set it
  caches
status: To Do
assignee:
  - TASK-2239
created_date: '2026-09-08 07:04'
updated_date: '2026-09-08 10:55'
labels:
  - code-review-rust
  - pattern
dependencies: []
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
- [ ] #1 A decision is recorded: either the freshness key is extended to cover the resolved member set, or the cache contract explicitly documents that the member list and canonical-manifest map are frozen for the entry's lifetime
- [ ] #2 If the key is extended, adding or removing a directory under a `prefix/*` glob causes the next `load_workspace_manifest` to re-resolve, and a test pins that (the existing `resolved_workspace_members_are_amortised_via_typed_manifest_cache` test is updated to assert the amortisation it actually still guarantees)
- [ ] #3 If the contract is documented instead, the `# Cache contract` **Invalidation** bullet in `manifest_cache.rs` names the two derived views and states that a host which can outlive a member-set change must call `ctx.refresh`
- [ ] #4 The reviewer rule at `manifest_cache.rs:74-76` ("do not add a daemon caller without first making the migration above") is extended to cover this staleness, so a future daemon caller cannot land without addressing it
<!-- AC:END -->
