---
id: TASK-1767
title: >-
  OWN-12: LoadedManifest derefs to CargoToml, silently exposing the raw glob
  spec the crate spends 60 lines warning about
status: To Do
assignee:
  - TASK-1993
created_date: '2026-08-27 11:20'
updated_date: '2026-08-28 14:12'
labels:
  - code-review-rust
  - idioms
dependencies: []
modified_files:
  - extensions-rust/about/src/query.rs
  - extensions-rust/about/src/identity/mod.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/query.rs:181-186` (`impl std::ops::Deref for LoadedManifest`), definition at `:123-138`

**What**: `LoadedManifest` is a three-field aggregate — `manifest: Arc<CargoToml>`, `resolved_members: Arc<Vec<String>>`, `canonical_member_manifests: Arc<OnceLock<HashMap<..>>>` — that implements `Deref<Target = CargoToml>`. OWN-12 reserves `Deref` for transparent wrappers and smart pointers; this is neither, and the consequence is concrete rather than stylistic.

The whole point of the ERR-1 / TASK-1076 design (documented across `:110-122`, `:141-154`, `:336-345`, and echoed in `units.rs:46-50`, `coverage_provider.rs:121-124`, `identity/mod.rs:69-77`) is that `manifest.workspace.members` holds the *unexpanded* glob spec (`["crates/*"]`) and the *only* correct read for consumers is `manifest.resolved_members()`. The `Deref` impl puts both spellings on the same receiver at every call site:

```rust
manifest.resolved_members()      // inherent — correct, the expanded list
manifest.workspace...members     // via Deref — the raw ["crates/*"] spec
```

`identity/mod.rs:74-77` shows the two mixed inside a single expression — `manifest.workspace` (Deref) guards `manifest.resolved_members().len()` (inherent). A reader cannot tell from the receiver which of the two lists a field access lands on, and reaching for the wrong one is a silent wrong answer (a `module_count` of 1 for a 40-crate workspace), not a compile error. The five separate comment blocks warning readers off `ws.members` are documentation compensating for an API that makes the wrong call reachable — the ERR-2/CL-3 "if documentation is required to prevent misuse, the API is fragile" case.

**Why it matters**: The failure mode is a plausible-looking wrong number with no diagnostic, and the guard against it is a comment rather than the type system. Every future consumer of `LoadedManifest` inherits the trap.

**Fix direction**: drop the `Deref` and expose the two or three fields that consumers actually need as named accessors (`package()`, `workspace()`, `resolved_members()`), so the raw spec is reached only through a deliberately-named method. Current Deref users are `units.rs:51`, `coverage_provider.rs:124`, `identity/mod.rs:51-52,74-77`, and the tests in `query.rs` — a small, mechanical migration.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 LoadedManifest no longer implements Deref<Target = CargoToml>
- [ ] #2 Consumers reach package/workspace data through named accessors on LoadedManifest, and the accessor exposing the unexpanded [workspace].members spec is named so its meaning is unambiguous at the call site
- [ ] #3 The redundant 'read resolved_members, not ws.members' comment blocks in units.rs, coverage_provider.rs and identity/mod.rs are removed or reduced, since the type now enforces what they were asking for
<!-- AC:END -->
