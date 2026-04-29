---
id: TASK-0570
title: >-
  READ-2: Maven scm single-line detector uses ends_with and accepts duplicate
  closing tags
status: Triage
assignee: []
created_date: '2026-04-29 05:04'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-java/about/src/maven/pom.rs:221-235`

**What**: match_section_open single-line shapes use line.starts_with of opening-scm-tag and line.ends_with of closing-scm-tag (and the same for licenses). A malformed line with duplicated closing tags matches and extracts the inner url; combined with the parser documented "no comment handling" caveat, this widens the surface to weird tolerance.

**Why it matters**: A stricter structural match is cheap and avoids quietly accepting clearly malformed POMs.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Single-line scm/licenses detection rejects lines containing more than one opener
- [ ] #2 Test covers a duplicated-tag line and asserts deterministic behaviour
<!-- AC:END -->
