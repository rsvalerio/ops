---
id: TASK-0561
title: 'ERR-1: parse_pom_xml logs at debug for NotFound, diverging from sister parsers'
status: Done
assignee:
  - TASK-0638
created_date: '2026-04-29 05:03'
updated_date: '2026-04-29 10:35'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-java/about/src/maven/pom.rs:74-82`

**What**: parse_pom_xml unconditionally emits tracing::debug for every read_to_string error including ErrorKind::NotFound, then returns Err(e). Every other manifest parser in the four about crates (go_mod, go_work, package_json, pyproject, units::read_workspace_members) explicitly skips the log when kind() == NotFound per TASK-0394.

**Why it matters**: Directories without a pom.xml emit debug logs on every about run, and the inconsistency makes TASK-0394 implicitly partial.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Reading a missing pom.xml emits no tracing event
- [ ] #2 Reading an unreadable (e.g. permission-denied) pom.xml still emits a tracing::debug event
- [ ] #3 Behaviour matches go_mod::parse / package_json::parse_package_json / parse_pyproject
<!-- AC:END -->
