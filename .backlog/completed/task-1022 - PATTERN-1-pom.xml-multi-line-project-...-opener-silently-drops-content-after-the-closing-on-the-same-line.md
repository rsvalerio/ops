---
id: TASK-1022
title: >-
  PATTERN-1: pom.xml multi-line <project ...> opener silently drops content
  after the closing > on the same line
status: Done
assignee: []
created_date: '2026-05-07 20:22'
updated_date: '2026-05-07 23:11'
labels:
  - code-review-rust
  - pattern
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-java/about/src/maven/pom.rs:106-118`

**What**: When a multi-line opener is detected (e.g. `<project xmlns=\"...\"\n         xsi:schemaLocation=\"...\">\n    <artifactId>foo</artifactId>`), the parser tracks `opener_pending = true` and on the line that contains `>`, it sets `started = true` and `continue`s — discarding any content on the same line *after* the closing `>`.

Real-world Maven formatters that fit the entire xmlns block onto one logical line and append `<artifactId>` immediately after the closing `>` would lose that artifactId:

```
<project xmlns=\"http://maven.apache.org/POM/4.0.0\"
         xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\"
         xsi:schemaLocation=\"...\"><artifactId>x</artifactId>
```

The existing test `parse_pom_multiline_project_opener` covers the case where the `>` line is the only content on its line — it does not catch this regression because the trailing fragment is empty.

**Why it matters**: Real Maven `pom.xml` files are usually pretty-printed (one element per line), so the impact is small. But the parser's contract has an asymmetry: a single-line `<project xmlns=\"...\"><artifactId>x</artifactId></project>` is rejected by `is_project_open` (which requires the line to equal `<project>` or to start with `<project ` and contain `>`), and the multi-line path drops content after `>`. The fix is to take `&line[after_gt..]` and either re-process it as a new top-level line or buffer it, mirroring `strip_xml_comments`'s loop shape.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 After detecting > on a multi-line opener line, recurse/loop into the post-> remainder so element tags on the same line are not dropped
- [ ] #2 Add a test where the closing > is followed by an <artifactId> tag on the same line — the artifactId must be captured
<!-- AC:END -->
