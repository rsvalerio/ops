---
id: TASK-2874
title: 'TEST-6: Compact one-line POM reads container children as project scalars'
status: To Do
assignee: []
created_date: '2026-11-18 22:24'
updated_date: '2026-11-20 13:26'
labels:
  - code-review-rust
  - TEST
dependencies: []
parent_task_id: 'TASK-2894'
modified_files:
  - extensions-java/about/src/maven/pom.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-java/about/src/maven/pom.rs` (`dispatch_opener_remainder`, `parse_top_level`)

**What**: Elements glued to the `<project>` opener's `>` are now dispatched as one started line. When a compact one-line POM also carries a container on that line, e.g. `<project><parent><version>9</version></parent><artifactId>x</artifactId></project>`, `match_section_open` does not match (the line neither equals nor starts with the container opener), so `parse_top_level` captures the parent's `<version>` (or a `<scm><url>` as the homepage, or a developer `<name>` as the project name).

**Why it matters**: A compact POM that carries `<parent>`, `<scm>`, `<developers>` or `<dependencies>` on the opener line can surface a wrong version, homepage or name on the About card. The module doc now states the limit, but the fields are still wrong rather than absent.

**Origin**: discovered during TASK-2848 while fixing TASK-2723.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A compact one-line POM with a container (parent, scm, developers, a skip section) glued to the opener does not fill project scalars from the container's children
- [ ] #2 Unit tests pin the parent-version and scm-url compact shapes
<!-- AC:END -->
