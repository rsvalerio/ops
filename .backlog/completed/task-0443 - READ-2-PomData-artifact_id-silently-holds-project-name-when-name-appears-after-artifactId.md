---
id: TASK-0443
title: >-
  READ-2: PomData::artifact_id silently holds project <name> when <name> appears
  after artifactId
status: Done
assignee:
  - TASK-0531
created_date: '2026-04-28 04:44'
updated_date: '2026-04-28 07:25'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-java/about/src/maven/pom.rs:186-188` (parse_top_level) and `pom.rs:25` (field declaration)

**What**: parse_top_level unconditionally overwrites data.artifact_id with <name> when it appears at top level — without the is_none() guard the artifactId branch uses. The field is named artifact_id but, post-parse, can hold either the artifact id or the human-readable project name (whichever appeared last). The parse_pom_basic test asserts artifactId == "My App" (the name), encoding this as intended.

**Why it matters**: Misleading field name + ordering-dependent behavior. A POM with <name> after <artifactId> keeps the name; with <name> before <artifactId>, the artifactId is preferred. Callers expecting an artifact coordinate get a display string. Future maintainers reading artifact_id will likely be confused.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either rename the field to reflect its actual semantics (e.g. display_name) or split into artifact_id and name with a documented preference applied at the provider boundary
- [ ] #2 Add a doc comment on the field explaining the precedence rule
- [ ] #3 Regression test: <artifactId>foo</artifactId> with no <name> yields the artifactId; <name> overrides only when intentionally preferred
<!-- AC:END -->
