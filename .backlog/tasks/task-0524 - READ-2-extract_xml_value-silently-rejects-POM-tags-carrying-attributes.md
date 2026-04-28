---
id: TASK-0524
title: 'READ-2: extract_xml_value silently rejects POM tags carrying attributes'
status: Done
assignee:
  - TASK-0531
created_date: '2026-04-28 06:52'
updated_date: '2026-04-28 07:25'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-java/about/src/maven/pom.rs:195`

**What**: extract_xml_value formats `<{tag}>` as a literal substring search. POM elements with attributes (e.g. `<artifactId xml:lang="en">camel</artifactId>`, namespace-prefixed elements) are not matched even though they are valid POM XML.

**Why it matters**: Module doc covers no comments / CDATA / multi-line, but does not call out attributes. POM elements with attributes silently produce None.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Document attribute-bearing tags as a known limitation OR match <tag followed by > / whitespace
- [ ] #2 If kept, doc-comment header lists this caveat alongside comment/CDATA limits
<!-- AC:END -->
