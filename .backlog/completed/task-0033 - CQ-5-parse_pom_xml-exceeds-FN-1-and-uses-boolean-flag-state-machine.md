---
id: TASK-0033
title: 'CQ-5: parse_pom_xml exceeds FN-1 and uses boolean-flag state machine'
status: Done
assignee: []
created_date: '2026-04-14 19:41'
updated_date: '2026-04-15 09:56'
labels:
  - rust-code-quality
  - complexity
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
extensions-java/about/src/lib.rs:148 — parse_pom_xml is 150 lines (FN-1 threshold: 50). Uses 5 boolean flags (in_modules, in_developers, in_developer, in_scm, in_licenses) as an implicit state machine, creating up to 5 nesting levels (FN-2 threshold: 4). Rules: FN-1, FN-2, CL-3. Refactoring: replace boolean flags with an enum State { TopLevel, Modules, Developers, Scm, Licenses } and extract section-parsing helpers (parse_modules_section, parse_developers_section, etc.).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 parse_pom_xml ≤50 lines
- [ ] #2 Boolean state flags replaced by enum-based state
- [ ] #3 Max nesting ≤4 levels
<!-- AC:END -->
