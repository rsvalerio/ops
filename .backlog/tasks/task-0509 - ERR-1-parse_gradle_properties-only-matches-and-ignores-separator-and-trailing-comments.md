---
id: TASK-0509
title: >-
  ERR-1: parse_gradle_properties only matches '=' and ignores ':' separator and
  trailing comments
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
**File**: `extensions-java/about/src/gradle.rs:98`

**What**: parse_gradle_properties only checks `version=` / `version =`. .properties files (used by Gradle via java.util.Properties) also accept `:` as separator and treat trailing `#`/`!` as comments. `version : 1.2` is not recognised; `version=1.2 # comment` captures the comment in the value.

**Why it matters**: Real-world gradle.properties files use the colon form and inline comments. About card silently drops version for those projects and captures noise for others.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Both version=1.2 and version : 1.2 are recognised
- [ ] #2 Trailing # / ! comments on the same line are stripped from the captured value
<!-- AC:END -->
