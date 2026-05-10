---
id: TASK-0562
title: >-
  PATTERN-1: Java about lib.rs docstring claims pom read errors propagate but
  provider unwrap_or_defaults
status: Done
assignee:
  - TASK-0640
created_date: '2026-04-29 05:03'
updated_date: '2026-04-29 11:50'
labels:
  - code-review-rust
  - pattern
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-java/about/src/lib.rs:7-11` (paired with `maven/mod.rs:27`)

**What**: lib.rs docstring claims: Read errors on pom.xml are surfaced through DataProviderError::computation_failed with the underlying I/O error in the message. The provider actually does parse_pom_xml(&cwd).unwrap_or_default(), so io::Error is silently coerced to an empty PomData and the dir-name fallback is returned.

**Why it matters**: Reviewers reading the docstring expect propagation; in practice unreadable manifests are indistinguishable from missing ones.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Either the docstring is rewritten to match the unwrap_or_default behaviour, or non-NotFound io::Errors are mapped to DataProviderError::computation_failed in MavenIdentityProvider::provide
- [x] #2 Final behaviour aligned with the gradle paragraph of the same docstring (which is accurate)
<!-- AC:END -->
