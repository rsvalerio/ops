---
id: TASK-0846
title: >-
  CL-3: Maven POM line scanner silently captures values inside XML comments and
  CDATA
status: Done
assignee: []
created_date: '2026-05-02 09:15'
updated_date: '2026-05-02 14:11'
labels:
  - code-review-rust
  - complexity
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-java/about/src/maven/pom.rs:1-128`

**What**: The module documents that comments and CDATA are not handled, but the consequence is silent corruption: a pom.xml containing `<!-- <artifactId>fake</artifactId> -->` on its own well-formed line will be captured as the project artifact ID. Same for `<scm><url>` inside a comment block. The parser treats these lines as structural.

**Why it matters**: This is a documented limitation that becomes a surprise in production. Real Maven repos do contain commented-out coordinates (release/SNAPSHOT swap pattern). Per CL-3, preconditions should be either enforced (skip lines starting with <!-- and tracking until -->) or asserted explicitly.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A skip-state for <!-- ... --> (multi-line aware) is added; commented <artifactId> is not captured
- [x] #2 Test pinning: a POM whose first <artifactId> is inside <!-- ... --> resolves to the second, real one
- [x] #3 Module doc updated to remove the no-comment-handling disclaimer once the skip is in place
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
parse_pom_xml now strips XML comments via strip_xml_comments(line, &mut in_comment) before tag matching. Multi-line aware (open on one line, close on another). Two regression tests pin the contract: parse_pom_commented_artifact_id_is_skipped (release/SNAPSHOT swap pattern) and parse_pom_multiline_comment_hides_inner_tags. Module doc updated to remove the no-comment-handling disclaimer; CDATA disclaimer kept (out of scope here).
<!-- SECTION:NOTES:END -->
