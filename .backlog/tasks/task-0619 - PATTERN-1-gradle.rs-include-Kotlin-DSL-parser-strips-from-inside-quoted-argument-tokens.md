---
id: TASK-0619
title: >-
  PATTERN-1: gradle.rs include() Kotlin-DSL parser strips ) from inside quoted
  argument tokens
status: Done
assignee:
  - TASK-0641
created_date: '2026-04-29 05:21'
updated_date: '2026-04-29 12:08'
labels:
  - code-review-rust
  - PATTERN
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-java/about/src/gradle.rs:86`

**What**: parse_gradle_settings handles `include("a", "b")` with `line.strip_prefix("include(").map(|r| r.trim_end_matches(|c: char| c.is_whitespace() || c == ")"))`. trim_end_matches strips every trailing match — `include("legacy)module")` would have characters inside the closing quote chewed off before extract_quoted_list.

**Why it matters**: PATTERN-1 — partial-input handler that looks total. Splitting once on structural ) makes intent clear and refuses pathological inputs gracefully.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Kotlin-DSL include parser splits once on structural ) rather than trim_end_matches(")")
- [ ] #2 Regression test covers an include whose quoted argument contains a ) character
- [ ] #3 Existing settings.gradle.kts tests pass
<!-- AC:END -->
