---
id: TASK-0120
title: >-
  DUP-4: Identity-provider monolith pattern repeats across Go/Node/Rust/Java
  about extensions
status: Done
assignee: []
created_date: '2026-04-19 18:51'
updated_date: '2026-04-19 19:44'
labels:
  - rust-code-review
  - duplication
  - architecture
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Files**:
- `extensions-rust/about/src/identity.rs` (~678 lines) — already flagged narrowly by TASK-0109, TASK-0112
- `extensions-go/about/src/lib.rs` (~508 lines)
- `extensions-node/about/src/lib.rs` (~465 lines)
- `extensions-java/about/src/maven.rs` (~449 lines), `extensions-java/about/src/gradle.rs`
- `extensions-python/about/src/lib.rs`

**What**: Each stack's `project_identity` provider mixes manifest parsing, field resolution, per-field `resolve_*` boilerplate, and `DataProvider` trait wiring in one module. The Rust variant is already flagged by TASK-0109 (7x resolve_field) and TASK-0112 (~107-line provide()). The same split-ability and duplication patterns exist in the Go, Node, Java, and Python stacks, but the previously filed tasks do not cover them, so a reviewer fixing Rust in isolation will leave the rest inconsistent.

**Why it matters**: (a) shared structural debt in at least 4 identity providers is cheaper to address as one pass than per-stack; (b) any refactor of `AboutFieldDef`/`base_about_fields` now has 4+ call-site shapes to keep in sync; (c) without a cross-stack task the Rust fix will ship solo and the inconsistency widens.

<!-- scan confidence: candidates to inspect -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Cross-stack refactor plan exists (either: a shared helper crate for manifest-driven identity providers, OR a documented decision to keep them separate with rationale)
- [ ] #2 If a shared helper is introduced, at least the Rust + one other stack are migrated in the same PR to prove the abstraction
- [x] #3 TASK-0109 / TASK-0112 are either closed by this work or explicitly re-scoped to Rust-only with a pointer to this task
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
## Cross-stack decision: keep identity providers per-stack (no shared helper crate)

**Decision.** Identity providers remain separate per stack (Rust/Go/Node/Java/Python).
TASK-0109 and TASK-0112 (Rust-only resolve_field collapse + provide() split) are the
reference refactor; each stack applies the same shape locally when it is touched.

**Rationale.**

1. *Manifest shapes diverge.* Cargo.toml has `[workspace.package]` inheritance with
   `InheritableField<T>` wrappers that fall back to a `WorkspacePackage`. Go's `go.mod`
   has no workspace-package inheritance. Node's `package.json` / workspaces, Maven's
   `pom.xml` (parent POM inheritance), Gradle's DSL, and Python's PEP 621 `pyproject.toml`
   each express inheritance differently. A shared helper would have to be generic over
   three dimensions (manifest type, package type, workspace-package type) plus per-field
   accessor pairs — the generic signature is longer than the specialized code it replaces.

2. *The duplicated piece is already small.* The `AboutFieldDef` / `base_about_fields`
   shared vocabulary already lives in `ops_core::project_identity` (CD-2, TASK-0030 —
   Done). What duplicates across stacks is the *shape* (`resolve_field` helper + field
   table + DuckDB metric queries), not the data. A 30-40-line module per stack is
   cheaper than a cross-stack abstraction that every stack has to bend to fit.

3. *Rate of change is low.* Identity field lists change rarely. `AboutFieldDef` is the
   shared contract; the per-stack resolvers are stable enough that drift cost is low.

**What the reference (Rust) refactor gives us.** The Rust provider in
`extensions-rust/about/src/identity.rs` now uses a local `r!` macro that collapses each
inheritable field to one line and splits `provide()` into `resolve_identity_fields` +
`query_identity_metrics` helpers. Each other stack can adopt the same two-helper
split + single-line field pattern when it is touched next; no new shared crate needed.

**Status of related tasks.**
- TASK-0109 — Done (Rust-only, resolve_field collapsed via `r!` macro).
- TASK-0112 — Done (Rust-only, provide() reduced to orchestrator under ~50 lines).
- This task (TASK-0120) — closed by this documented decision.
<!-- SECTION:NOTES:END -->
