---
id: TASK-0508
title: 'ERR-1: parse_gradle_settings does not handle method-call form ''include a, b'''
status: Done
assignee:
  - TASK-0531
created_date: '2026-04-28 06:51'
updated_date: '2026-04-28 07:25'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-java/about/src/gradle.rs:75`

**What**: Gradle Groovy DSL accepts `include \'a\', \'b\', \'c\'` to register multiple subprojects. parse_gradle_settings only extracts the first quoted value after `include ` (and `include(...)` likewise treats the parenthesised content as a single quoted value).

**Why it matters**: Comma-separated includes silently lose all but the first subproject from module_count, undercounting workspaces.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 include 'a', 'b' and include('a', 'b') both register two subprojects
- [ ] #2 Existing single-include tests still pass
<!-- AC:END -->
