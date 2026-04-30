---
id: TASK-0687
title: >-
  PATTERN-1: gradle Kotlin-DSL include() parser drops first call when two appear
  on the same line
status: Done
assignee:
  - TASK-0736
created_date: '2026-04-30 05:16'
updated_date: '2026-04-30 10:13'
labels:
  - code-review-rust
  - patterns
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-java/about/src/gradle.rs:87-96`

**What**: The Kotlin-DSL `include("a", "b")` matcher uses `rsplit_once(')')` after stripping a trailing `//` comment. A second `include(...)` on the same line (`include("a"); include("b")`, or chained DSL calls common in apply configurations) leaves the parser keeping only the *last* paren-delimited segment, dropping the first include.

**Why it matters**: Two-include-per-line is unusual but legal Kotlin DSL; on encounter the parser silently undercounts subprojects and drops modules from the About card without any tracing event.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Either iterate all include(...) occurrences on the line, or document and tracing::debug! when an include( appears after the first ) on the same line
- [x] #2 Test pins behaviour for include("a"); include("b")\n
<!-- AC:END -->
