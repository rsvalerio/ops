---
id: TASK-0441
title: >-
  ERR-1: Gradle include parser drops every member after the first in multi-arg
  form
status: Done
assignee:
  - TASK-0531
created_date: '2026-04-28 04:44'
updated_date: '2026-04-28 07:25'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-java/about/src/gradle.rs:75-83` (and extract_quoted at line 136-143)

**What**: extract_quoted requires the entire input to start AND end with matching quotes. Gradle DSL allows `include \'a\', \'b\', \'c\'` (or include("a", "b")) on a single line; extract_quoted("\'a\', \'b\', \'c\'") returns None because the trailing char is not a quote. The result is that the entire include statement is silently dropped, so module_count is wrong for any settings.gradle using this idiom.

**Why it matters**: Multi-arg include is common in real Gradle projects (Spring Boot, Micronaut, etc.). The bug is silent — no warning, just an undercount in the project card. There is no test for the multi-arg form.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 parse_gradle_settings extracts every quoted token from include 'a', 'b' and include("a", "b") forms
- [ ] #2 Regression tests cover both the space-and-comma form and the parenthesized comma form, asserting includes.len() == 2+
- [ ] #3 Trailing/inline comments (include 'core' // comment) do not break extraction
<!-- AC:END -->
