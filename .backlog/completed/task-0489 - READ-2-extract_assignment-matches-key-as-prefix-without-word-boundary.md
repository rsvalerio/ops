---
id: TASK-0489
title: 'READ-2: extract_assignment matches key as prefix without word boundary'
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
**File**: `extensions-java/about/src/gradle.rs:127`

**What**: extract_assignment uses line.starts_with(key) and then strips key.len() bytes, so a line like `rootProject.nameOverride = "x"` matches when called with key `rootProject.name` and yields the value of the unrelated property.

**Why it matters**: Silent misattribution: the project identity may be populated from an unrelated assignment whose key happens to share a prefix with the queried key (e.g. rootProject.nameSpace, descriptionText, versioned). No diagnostic surfaces this.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 extract_assignment requires that the character immediately after the key is = or whitespace before =
- [ ] #2 Test added covering rootProject.nameOverride = "x" returning None when querying rootProject.name
- [ ] #3 Test added covering descriptionText = "x" returning None when querying description
<!-- AC:END -->
