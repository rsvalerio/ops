---
id: TASK-0319
title: >-
  FN-1: parse_pom_xml is 82 lines mixing state-machine dispatch, section
  transitions, and per-field extraction
status: Done
assignee:
  - TASK-0327
created_date: '2026-04-24 08:54'
updated_date: '2026-04-25 13:44'
labels:
  - rust-code-review
  - complexity
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: extensions-java/about/src/maven.rs:71-152

**What**: Single fn combines start/end tag detection, section match, and field assignment — three abstraction levels.

**Why it matters**: Exceeds FN-1 threshold; hard to add a new section without touching unrelated code.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extract a handle_section(section, line, data) helper per variant
- [ ] #2 parse_pom_xml becomes the top-level driver under 30 lines
<!-- AC:END -->
