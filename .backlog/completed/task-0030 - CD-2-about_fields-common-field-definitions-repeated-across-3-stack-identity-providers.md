---
id: TASK-0030
title: >-
  CD-2: about_fields() common field definitions repeated across 3 stack identity
  providers
status: Done
assignee: []
created_date: '2026-04-14 19:35'
updated_date: '2026-04-15 09:56'
labels:
  - rust-code-duplication
  - DUP-2
  - DUP-3
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Location**: extensions-go/about/src/lib.rs:42-85, extensions-java/about/src/lib.rs:66-87 (JAVA_ABOUT_FIELDS), extensions-rust/about/src/identity.rs:22-79
**Anchor**: fn about_fields / JAVA_ABOUT_FIELDS
**Impact**: All 3 stack identity providers (Go, Java, Rust) define overlapping about_fields() returning mostly the same AboutFieldDef entries. 7 fields are identical across all providers: project, modules, code, files, authors, repository, coverage, languages — with identical id, label, and description values. The Java extension already partially addressed this by extracting JAVA_ABOUT_FIELDS as a const array, but the Go and Rust extensions define them inline. The Rust provider adds 3 stack-specific extras (homepage, msrv, dependencies). This is ~30 lines of identical field definitions repeated 3 times.

Fix: define BASE_ABOUT_FIELDS in ops_extension (or ops_core::project_identity) as a shared const slice. Each provider's about_fields() extends the base with stack-specific extras. Java's existing JAVA_ABOUT_FIELDS pattern is a good model — just needs to be promoted to shared code.

DUP-2: 3+ structurally similar functions. DUP-3: 3+ occurrences of repeated AboutFieldDef patterns.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Common about fields (project, modules, code, files, authors, repository, coverage, languages) are defined once and shared across all identity providers
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
CD audit re-confirmation: about_fields() verified in 3 providers. 7 fields (project, modules, code, files, authors, repository, coverage, languages) have verbatim identical AboutFieldDef structs across Go (extensions-go/about/src/lib.rs), Rust (extensions-rust/about/src/identity.rs), and Java (extensions-java/about/src/lib.rs). Java already partially deduplicates via java_about_fields() shared between Maven and Gradle providers. Additionally, all providers end provide() with identical serde_json::to_value(&identity).map_err(DataProviderError::from) terminal line.
<!-- SECTION:NOTES:END -->
