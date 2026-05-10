---
id: TASK-0624
title: >-
  READ-2: parse_gradle_settings does not strip inline comments before matching
  rootProject.name
status: Done
assignee:
  - TASK-0641
created_date: '2026-04-29 05:21'
updated_date: '2026-04-29 12:09'
labels:
  - code-review-rust
  - READ
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-java/about/src/gradle.rs:75`

**What**: parse_gradle_settings uses extract_assignment and a manual strip_trailing_comment only on the Groovy bare-include form (gradle.rs:87). The `rootProject.name = "x" // some comment` line passes through unchanged because extract_quoted short-circuits at the second quote, but a future caller switch to greedy matching would extract a stray quote-bounded slice. Implicit reliance is fragile and not asserted by any test.

**Why it matters**: Comment-stripping is done inconsistently — some forms strip, others rely on quoted-value short-circuiting.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Test pins rootProject.name = "x" // trailing comment to extract exactly x
- [ ] #2 Either closure strips trailing // comments before key matching, or test+comment documents short-circuit contract
<!-- AC:END -->
