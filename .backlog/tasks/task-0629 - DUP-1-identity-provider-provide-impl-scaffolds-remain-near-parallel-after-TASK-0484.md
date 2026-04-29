---
id: TASK-0629
title: >-
  DUP-1: identity-provider provide impl scaffolds remain near-parallel after
  TASK-0484
status: Triage
assignee: []
created_date: '2026-04-29 05:22'
labels:
  - code-review-rust
  - DUP
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-go/about/src/lib.rs:55`

**What**: TASK-0484 (Done) introduced provide_identity_from_manifest and migrated Python (extensions-python/about/src/lib.rs:64-95). Go (extensions-go/about/src/lib.rs:55-87), Node (extensions-node/about/src/lib.rs:59-94), Maven (extensions-java/about/src/maven/mod.rs:25-48), Gradle (extensions-java/about/src/gradle.rs:23-56) still call build_identity_value directly, each with `let cwd = ctx.working_directory.clone();` + parse_<manifest>(&cwd).unwrap_or_default() + ParsedManifest { ..::default() } shape. Migration complete on one of five providers.

**Why it matters**: TASK-0484 was filed precisely for this duplication and was closed Done after migrating one provider. Pattern persists in four out of five sister crates — exactly the Done-with-note shortcut failure mode.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Go, Node, Maven, Gradle providers migrate to provide_identity_from_manifest (or thin variant)
- [ ] #2 All provide() impls in the four about crates use the same call shape
- [ ] #3 Existing tests pass with no behaviour change
<!-- AC:END -->
