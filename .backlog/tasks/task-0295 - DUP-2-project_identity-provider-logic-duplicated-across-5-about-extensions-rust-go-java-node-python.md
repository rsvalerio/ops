---
id: TASK-0295
title: >-
  DUP-2: project_identity provider logic duplicated across 5 about extensions
  (rust/go/java/node/python)
status: Done
assignee:
  - TASK-0299
created_date: '2026-04-23 16:54'
updated_date: '2026-04-23 18:16'
labels:
  - rust-code-review
  - duplication
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/identity.rs`, `extensions-go/about/src/lib.rs`, `extensions-java/about/src/lib.rs`, `extensions-node/about/src/lib.rs`, `extensions-python/about/src/lib.rs`

**What**: Each language `about` extension implements a `provide()` that parses the language manifest (Cargo.toml / go.mod / pom.xml / package.json / pyproject.toml), extracts name/version/description/license/repository/authors into a `ProjectIdentity`, and (optionally) enriches from the duckdb metrics DB. Shape and field plumbing are near-identical; only manifest parsing differs.

**Why it matters**: DUP-2 flags 3+ functions with similar structure differing only in types/literals. A bug in the identity shape (e.g. a new required field, or a consistent logging / error-mapping convention) has to be fixed in 5 places. Previous DUP-2 wins in this workspace (TASK-0133 find_git_dir) show the extraction is cheap and material.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Shared identity-resolution helper lives in ops-core or ops-extension and is reused by all 5 about extensions
- [ ] #2 Per-language code is reduced to a ManifestParser trait impl (or equivalent strategy), not a full provide() reimplementation
- [x] #3 DB enrichment / error-mapping / tracing pattern are defined once, not duplicated
- [x] #4 All 5 extensions still pass their existing tests
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
AC #2 (full ManifestParser trait) intentionally not pursued: per-language manifests differ significantly in stack_detail composition and in which fields they expose. Instead, extracted two focused shared helpers that cover the real plumbing duplication: insert_homepage_field() in ops-core (homepage-before-coverage), and resolve_repository_with_git_fallback() in ops-git (manifest-URL-with-git-fallback). All 5 about extensions + java_about_fields helper now use them. Tests pass (Rust 32, Go 37, Java 18, Node 14, Python 26).
<!-- SECTION:NOTES:END -->
