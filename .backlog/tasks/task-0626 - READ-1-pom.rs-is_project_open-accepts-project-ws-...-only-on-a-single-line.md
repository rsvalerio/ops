---
id: TASK-0626
title: 'READ-1: pom.rs::is_project_open accepts <project<ws>...> only on a single line'
status: Triage
assignee: []
created_date: '2026-04-29 05:22'
labels:
  - code-review-rust
  - READ
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-java/about/src/maven/pom.rs:121`

**What**: is_project_open (pom.rs:121-131) requires the `<project ...>` opening tag — including any namespace attributes — to fit on a single line. Real-world Maven pom.xml formatters often emit attributes split across multiple lines. Parser silently never enters scan loop; parse_pom_xml returns empty PomData::default() served as "missing pom" via unwrap_or_default.

**Why it matters**: Silent zero-result on a perfectly valid pom.xml is correctness-adjacent. Extending opener to absorb continuation lines until first > closes the user-visible failure mode.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 parse_pom_xml supports multi-line <project ... > openers, OR module docstring lists this as known unsupported with workaround
- [ ] #2 Test fixture pins chosen behaviour with multi-line <project> opener
<!-- AC:END -->
