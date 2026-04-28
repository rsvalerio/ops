---
id: TASK-0438
title: >-
  ERR-2: MavenIdentityProvider hard-fails on missing/unreadable pom.xml instead
  of falling back
status: Done
assignee:
  - TASK-0531
created_date: '2026-04-28 04:43'
updated_date: '2026-04-28 07:25'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-java/about/src/maven/mod.rs:27-32` (and `extensions-java/about/src/maven/pom.rs:46-52`)

**What**: provide() propagates any parse_pom_xml error — including ErrorKind::NotFound — as DataProviderError::computation_failed. Every other identity provider in the workspace (go, python, node, rust) calls parse_*().unwrap_or_default() and falls back to directory-name + ParsedManifest::default(). The Maven provider's own test maven_provider_provide_no_pom enshrines the divergent behavior as expected.

**Why it matters**: Inconsistent contract across stacks. If a Java project also gets the Maven provider registered (e.g. nested project, wrong cwd, transient I/O failure), users see a DataProviderError rather than the documented graceful identity fallback. The module-level rustdoc (lib.rs:7-12) also explicitly contrasts Maven hard failure with Gradle silent treatment, acknowledging the inconsistency.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 MavenIdentityProvider::provide returns Ok with directory-name fallback when pom.xml is missing (mirroring sibling providers)
- [ ] #2 Non-NotFound I/O errors and parse anomalies are logged via tracing but still allow fallback identity construction
- [ ] #3 Test maven_provider_provide_no_pom is updated to assert Ok with default-shaped identity
<!-- AC:END -->
