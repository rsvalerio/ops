---
id: TASK-0817
title: >-
  READ-2: parse_gradle_settings closure scan body is 38 lines mixing assignment
  extraction, Kotlin DSL chained-include loop, and Groovy form
status: Done
assignee:
  - TASK-0823
created_date: '2026-05-01 06:03'
updated_date: '2026-05-01 09:21'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-java/about/src/gradle.rs:68-118`

**What**: The scan closure is the bulk of the function and itself mixes three parser shapes. With FN-2 nesting at 4 levels, it is the densest section in the crate. Each Kotlin-DSL fix (TASK-0619, TASK-0687) has piled an extra branch into the same closure.

**Why it matters**: Nesting now obscures the simpler Groovy paths and rejects future shapes. Extracting a parse_include_line helper restores symmetry with the Maven section dispatch.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extract parse_include_line(line, includes) — handles include(, bare include, and chained Kotlin forms
- [ ] #2 Resulting parse_gradle_settings body <=25 lines
- [ ] #3 Existing tests green
<!-- AC:END -->
