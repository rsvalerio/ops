---
id: TASK-0493
title: 'READ-2: parse_pom_xml accepts <projectFoo> as the opening project tag'
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
**File**: `extensions-java/about/src/maven/pom.rs:70-72`

**What**: The opener check is `line.starts_with("<project")`, which matches any element whose name begins with project (e.g. <projectVersion>). For closer the parser correctly compares to </project> exactly, creating an asymmetry: a stray top-level <projectInfo> triggers started=true even before the real <project> opens.

**Why it matters**: Minor robustness gap. A pom.xml with a leading <projectVersion>-style line would flip parsing into the body before <project>, causing bogus extraction. Cheap to tighten.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Opening check matches <project> or <project ... > (with attribute whitespace) only, not arbitrary <project* prefixes
- [ ] #2 Test added covering a leading <projectInfo> line not flipping started to true
<!-- AC:END -->
