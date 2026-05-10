---
id: TASK-0481
title: >-
  READ-2: match_section_open has redundant clause and misses single-line
  scm/licenses blocks
status: Done
assignee:
  - TASK-0531
created_date: '2026-04-28 05:48'
updated_date: '2026-04-28 07:25'
labels:
  - code-review-rust
  - READ
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: extensions-java/about/src/maven/pom.rs:157-171

**What**: match_section_open uses 'line == "<scm>" || line.starts_with("<scm>")' (and the same for <licenses>); the second clause subsumes the first. More importantly, when <scm>...</scm> appears on a single line (common for short SCM blocks) the parser enters Scm state but never sees </scm> as a stand-alone line — content after </scm> on the same line is lost and a single-line <scm><url>x</url></scm> is never extracted.

**Why it matters**: Real parsing gap producing an empty repository field for tersely-formatted POMs and undocumented in the Known limits header.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Drop the redundant equality arm or restructure so opening/closing tags on the same line are handled (and similarly for <licenses>)
- [ ] #2 Add tests for <scm><url>https://example.com</url></scm> on one line and the same pattern with <licenses>
<!-- AC:END -->
