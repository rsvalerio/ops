---
id: TASK-0618
title: >-
  API-1: parse_pom_xml returns Result<PomData, io::Error> while sister parsers
  return Option<T>
status: Triage
assignee: []
created_date: '2026-04-29 05:21'
labels:
  - code-review-rust
  - API
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-java/about/src/maven/pom.rs:74`

**What**: parse_pom_xml is the only top-level manifest parser in the four about crates returning Result<_, io::Error>. go_mod::parse, go_work::parse_use_dirs, parse_pyproject, parse_package_json, parse_gradle_settings, parse_gradle_properties all return Option<T> and log internally. The sole caller (maven/mod.rs:27) discards the error via unwrap_or_default(), so the Result carries no value at the call site.

**Why it matters**: A Result whose only consumer is unwrap_or_default() is signal noise. Aligning with sister parsers makes the cross-stack pattern obvious.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 parse_pom_xml returns Option<PomData> consistent with sister parsers
- [ ] #2 unwrap_or_default at maven/mod.rs:27 continues to work
- [ ] #3 All existing pom tests pass
<!-- AC:END -->
