---
id: TASK-0812
title: >-
  READ-2: strip_properties_comment treats # and ! anywhere on the line as a
  comment, not just at start
status: Triage
assignee: []
created_date: '2026-05-01 06:03'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-java/about/src/gradle.rs:283-291`

**What**: s.find(#).into_iter().chain(s.find(!)).min() cuts the value at the first # or ! in the whole string. A real version=1.0!beta becomes 1.0. Java .properties only treats these as comment introducers at the beginning of a line.

**Why it matters**: parse_gradle_properties reads version from gradle.properties; users with !/# inside a version string lose the suffix silently. Diverges from Java spec semantics.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Only strip when the comment marker is at column 0 of the (already-trimmed) line, or preceded by whitespace
- [ ] #2 Tests for version=1.0!beta, version=1.0#snapshot, and # version=2.0 (a real comment line, already filtered upstream)
<!-- AC:END -->
