---
id: TASK-0491
title: >-
  READ-2: handle_licenses captures stray <name> elements without an <license>
  wrapper guard
status: Done
assignee:
  - TASK-0531
created_date: '2026-04-28 06:09'
updated_date: '2026-04-28 07:25'
labels:
  - code-review-rust
  - read
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-java/about/src/maven/pom.rs:146-153`

**What**: Inside <licenses>, handle_licenses extracts <name> from any line, unlike handle_developers which tracks an in_developer flag. Stray <name> (e.g. inserted by a sibling element or malformed POM) is captured as the license name.

**Why it matters**: Inconsistent with handle_developers and produces wrong license values for malformed or unusual POMs without any diagnostic. Calibrated as low because well-formed POMs are unaffected.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 PomSection::Licenses tracks in_license analogous to Developers
- [ ] #2 handle_licenses only extracts <name> when in_license is true
- [ ] #3 Test added covering <licenses><name>stray</name>...</licenses> not setting license
<!-- AC:END -->
