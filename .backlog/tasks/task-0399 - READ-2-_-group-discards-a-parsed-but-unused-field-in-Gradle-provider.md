---
id: TASK-0399
title: 'READ-2: _ = group; discards a parsed-but-unused field in Gradle provider'
status: Done
assignee:
  - TASK-0417
created_date: '2026-04-26 09:41'
updated_date: '2026-04-27 19:40'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-java/about/src/gradle.rs:39`

**What**: parse_gradle_build extracts group and the provider parses it, then immediately discards it via let _ = group; with no assignment to ProjectIdentity and no comment.

**Why it matters**: Either group should be surfaced or the parser should not extract it. The dead binding is a TODO without a TODO; READ-4 requires explaining why.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either delete the group field from GradleBuild and its parser branch, or add a comment explaining what blocks surfacing it
- [ ] #2 If kept, add a test asserting group is parsed correctly so the parser logic is not silently broken
<!-- AC:END -->
