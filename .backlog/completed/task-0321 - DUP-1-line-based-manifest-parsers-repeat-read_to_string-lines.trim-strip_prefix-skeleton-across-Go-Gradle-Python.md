---
id: TASK-0321
title: >-
  DUP-1: line-based manifest parsers repeat read_to_string + lines.trim +
  strip_prefix skeleton across Go/Gradle/Python
status: Done
assignee:
  - TASK-0327
created_date: '2026-04-24 08:54'
updated_date: '2026-04-25 13:47'
labels:
  - rust-code-review
  - duplication
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: extensions-go/about/src/lib.rs:98-126 and 134-167; extensions-java/about/src/gradle.rs:68-115; extensions-python/about/src/lib.rs

**What**: ≥5-line identical shape across three manifest parsers.

**Why it matters**: DUP-1; a shared helper line_scan(path, &[(prefix, handler)]) would unify behavior and make cross-extension fixes single-point.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Helper extracted into a shared module (e.g. crates/core or extensions-common)
- [ ] #2 Go/Gradle/Python parsers migrated with unchanged behavior
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Python migration N/A: extensions-python/about/src/lib.rs parses pyproject.toml via toml::from_str (structured), not the line-based skeleton. Helper for_each_trimmed_line added to ops_core::text and applied to go.mod, go.work, settings.gradle(.kts), gradle.properties, build.gradle(.kts).
<!-- SECTION:NOTES:END -->
