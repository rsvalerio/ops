---
id: TASK-0630
title: 'PATTERN-1: extract_quoted_list accepts unbalanced quotes without warning'
status: Done
assignee:
  - TASK-0642
created_date: '2026-04-29 05:22'
updated_date: '2026-04-29 12:51'
labels:
  - code-review-rust
  - PATTERN
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-java/about/src/gradle.rs:184`

**What**: extract_quoted_list (gradle.rs:184-202) silently returns when it encounters a token that does not begin with a quote OR a quote that is never closed. Tokens accumulated so far are kept; malformed remainder dropped without diagnostic. `include "core", noise` returns just ["core"] with no tracing::debug indicating the parser bailed early.

**Why it matters**: Other line-based parsers in this crate log at debug when they reject a line. Bailing silently makes a partially-valid include look intentional.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 extract_quoted_list logs at tracing::debug when it bails on malformed remainder OR returns sentinel caller surfaces
- [x] #2 Test covers unbalanced-quote and bare-token cases and pins chosen behaviour
<!-- AC:END -->
