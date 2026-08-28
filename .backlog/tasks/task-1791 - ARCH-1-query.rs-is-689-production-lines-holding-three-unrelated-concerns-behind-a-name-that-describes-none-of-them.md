---
id: TASK-1791
title: >-
  ARCH-1: query.rs is 689 production lines holding three unrelated concerns
  behind a name that describes none of them
status: To Do
assignee:
  - TASK-1993
created_date: '2026-08-27 11:24'
updated_date: '2026-08-28 14:12'
labels:
  - code-review-rust
  - architecture
dependencies: []
modified_files:
  - extensions-rust/about/src/query.rs
  - extensions-rust/about/src/lib.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/query.rs` (1419 lines total; `:1-689` production, `:690-1419` tests)

**What**: One module, three concerns with no coupling between the first and the third:

1. **`:25-334` — a bounded LRU cache (310 lines).** `MAX_TYPED_MANIFEST_CACHE_ENTRIES`, `ManifestFreshness`, `TypedManifestEntry`, `TypedManifestCache` + `evict_lru`, `cargo_toml_freshness`, the `OnceLock` static, and `lock_typed_manifest_cache` with its poison-recovery counter. This is a general-purpose caching component; nothing in it is about Cargo workspaces beyond the mtime+len key.
2. **`:336-497` — manifest loading.** `LoadedManifest`, its `Deref`, `resolved_members`, `canonical_member_manifests`, `log_manifest_load_failure`, `is_manifest_missing`, `load_workspace_manifest`.
3. **`:499-688` — workspace-member glob resolution (190 lines).** `resolved_workspace_members`, `MemberShape`, `classify_member`, `expand_member_glob`, `is_unsupported_glob`, `contains_unsupported_glob_meta`, `member_path_is_workspace_safe`. Pure functions over a `CargoToml` and a root path; they never touch the cache.

ARCH-1's module red flags all fire: >500 lines, unrelated concerns in one file, and no cohesive theme. ARCH-10's naming point compounds it — `query.rs` names none of the three (nothing here issues a query; the `DuckDb` queries live in `metrics.rs`, `deps_provider.rs`, and `coverage_provider.rs`).

The split is already precedented and already named in this file's own comments. `:264-267` instructs the reader to *"Keep this comment in sync with `extensions/about/src/manifest_cache.rs`"* — the sibling crate keeps exactly this concern in a file called `manifest_cache.rs`. Meanwhile `member_path_is_workspace_safe` and `resolved_workspace_members` are the crate's public surface for sibling extensions (`lib.rs:41` re-exports the latter for `create-review-tasks-rust`), so they are the part with real external consumers and the part currently hardest to find.

**Why it matters**: the cache section carries the crate's most delicate concurrency contract — a 25-line reviewer directive at `:248-272` ("do not add a daemon caller without first making the migration above") that a reader only encounters if they scroll past it on the way to the glob expander. Separating the concerns puts that contract at the top of the file it governs. It also makes the 730-line test module tractable: cache tests, loader tests, and glob tests are currently interleaved in one `mod tests`.

**Fix direction**: `manifest_cache.rs` (concern 1, with the CONC-7 contract as its module doc and the cache tests), `members.rs` (concern 3, with the glob and path-safety tests), and keep concern 2 in a file named for what it does — `manifest.rs` or `loader.rs` — rather than `query.rs`. Update the `lib.rs:27-31` module list and the `lib.rs:41` re-export accordingly.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The typed-manifest cache (MAX_TYPED_MANIFEST_CACHE_ENTRIES through lock_typed_manifest_cache) lives in its own module, with the CONC-7 / TASK-1163 concurrency contract as that module's doc comment
- [ ] #2 Workspace-member glob resolution and path safety (resolved_workspace_members, classify_member, expand_member_glob, member_path_is_workspace_safe) live in their own module
- [ ] #3 No remaining module in the crate exceeds 500 production lines, and no module is named query.rs unless it issues queries
- [ ] #4 Tests move with the code they cover, and lib.rs module declarations plus the resolved_workspace_members re-export are updated with no change to the crate's public API
<!-- AC:END -->
