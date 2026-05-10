---
id: TASK-0391
title: >-
  ARCH-1: extensions-java/about/src/maven.rs mixes provider, line-based XML
  parser, section state machine, and tests in 463 lines
status: Done
assignee:
  - TASK-0417
created_date: '2026-04-26 09:40'
updated_date: '2026-04-27 19:59'
labels:
  - code-review-rust
  - architecture
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-java/about/src/maven.rs:1`

**What**: maven.rs contains MavenIdentityProvider, PomData, the PomSection state machine, six handle_* functions, match_section_open, parse_top_level, extract_xml_value, and a sizable test module.

**Why it matters**: Coupling provider with hand-rolled line-oriented XML parser makes both harder to evolve and test. A robust replacement (e.g., quick-xml) becomes a multi-file refactor instead of a drop-in module swap.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extract the parser into extensions-java/about/src/maven/pom.rs (or pom_xml.rs) exposing parse_pom_xml and PomData; keep maven.rs only as the DataProvider impl
- [ ] #2 Document the parser known limits (no comment handling, no CDATA, no nested duplicate elements) in a module-level doc comment
<!-- AC:END -->
