---
id: TASK-1762
title: >-
  CL-3: Rust about providers assume ctx.working_directory IS the workspace root,
  but the manifest is discovered by an ancestor walk
status: Triage
assignee: []
created_date: '2026-08-27 11:20'
labels:
  - code-review-rust
  - idioms
dependencies: []
modified_files:
  - extensions-rust/about/src/query.rs
  - extensions-rust/about/src/units.rs
  - extensions-rust/about/src/coverage_provider.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/query.rs:346` (`load_workspace_manifest`), specifically `:355` (freshness stat), `:433` (root discovery), `:452-455` (glob resolution)

**What**: `load_workspace_manifest` discovers the workspace root by walking *ancestors* of the cwd (`find_workspace_root_strict(ctx.working_directory.as_path())` at `:433`) and loads the manifest from that discovered root. It then throws the discovered root away and resolves everything else against `ctx.working_directory`:

- `:355` `cargo_toml_freshness(ctx.working_directory.as_path())` stats `<cwd>/Cargo.toml`, not `<discovered_root>/Cargo.toml`.
- `:452-455` `resolved_workspace_members(&manifest, ctx.working_directory.as_path())` expands `crates/*` globs by `read_dir`-ing `<cwd>/crates`, not `<root>/crates`.
- Downstream, `units.rs:103` (`cwd.join(member).join("Cargo.toml")`), `query.rs:172` (`canonical_member_manifests`), and `coverage_provider.rs:148/158` (`query_crate_coverage(db, &member_strs, cwd_str)` and `resolve_crate_display_name(member, &cwd)`) all join member paths onto the same cwd.

`ctx.working_directory` is the live process cwd, not a normalised project root: `crates/cli/src/main.rs:205` sets it from `std::env::current_dir()` and hands it to `Context::from_cwd_arc` (`crates/runner/src/command/mod.rs:257`) unchanged. So the invariant "cwd == workspace root" is nowhere enforced — it is simply assumed, and the ancestor walk at `:433` is proof the code already knows cwd may sit below the root.

Consequences when `ops about` is run from a subdirectory of a Cargo workspace (a member crate, `src/`, anywhere):

1. Glob members silently expand to nothing (`read_dir(<cwd>/crates)` fails, `expand_member_glob` warns and returns empty), so `module_count`, `project_units`, and `project_coverage` units all come back empty or wrong.
2. Literal members resolve to non-existent paths, so `read_crate_metadata` gets `NotFound` (which is deliberately silent) and every unit loses its name/version/description.
3. The manifest cache freshness key stats a `Cargo.toml` that is not the one that was parsed. If `<cwd>/Cargo.toml` does not exist, `cargo_toml_freshness` returns `None` and the `(None, _) => true` arm at `:371-374` pins the entry as permanently fresh — the cached manifest is then never invalidated for the process lifetime, defeating the entire TASK-0843 / TASK-1198 mtime+len freshness design.
4. Two cwds under the same workspace occupy two cache slots holding the same manifest.

**Why it matters**: This is a silent wrong-answer path, not a crash. `ops about` from a subdirectory reports a plausible-looking but empty/incorrect project view with no error, and the freshness machinery that the surrounding 60 lines of comments carefully justify is inert in exactly that case. It is invisible to the test suite because every test constructs `Context::test_context(root)` with cwd already equal to the workspace root, so the assumed precondition is never violated in a test.

**Fix direction**: resolve the workspace root once, and thread that `PathBuf` — not `ctx.working_directory` — through the freshness stat, `resolved_workspace_members`, `canonical_member_manifests`, and the provider-side joins. Key the cache by the resolved root so sibling cwds share one entry. If cwd really is required to equal the root, make that a checked precondition with a typed error rather than an undocumented assumption (CL-3).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 load_workspace_manifest resolves the workspace root once and uses that root (not ctx.working_directory) for the Cargo.toml freshness stat, for resolved_workspace_members glob expansion, and for LoadedManifest::canonical_member_manifests
- [ ] #2 units.rs and coverage_provider.rs join member paths onto the resolved workspace root rather than ctx.working_directory (including the SQL key passed to query_crate_coverage and the workspace_root passed to resolve_crate_display_name)
- [ ] #3 The typed-manifest cache is keyed by the resolved workspace root so two cwds inside the same workspace share one entry and one freshness key
- [ ] #4 A regression test runs the identity/units/coverage providers with Context::test_context pointed at a SUBDIRECTORY of a glob workspace and asserts the same members/module_count as running from the root
<!-- AC:END -->
