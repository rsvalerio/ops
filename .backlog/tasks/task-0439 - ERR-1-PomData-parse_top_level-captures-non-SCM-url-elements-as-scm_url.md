---
id: TASK-0439
title: 'ERR-1: PomData parse_top_level captures non-SCM <url> elements as scm_url'
status: Done
assignee:
  - TASK-0531
created_date: '2026-04-28 04:43'
updated_date: '2026-04-28 07:25'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-java/about/src/maven/pom.rs:189-191` (parse_top_level)

**What**: When section == TopLevel, every line is checked for <url>...</url> and assigned to scm_url if unset. The state machine only tracks <scm>, <licenses>, <modules>, <developers>. A <url> inside <organization>, <issueManagement>, <ciManagement>, <distributionManagement>, <parent>, etc. is therefore captured as the SCM URL when it appears before <scm> in the file.

**Why it matters**: Repository field is surfaced in ProjectIdentity and shown to users; capturing the organization homepage or CI URL as the source repository is a silent correctness bug. The module docstring already warns "no nested duplicate elements"; this finding is to harden against the most common wrong sections, not to introduce a full XML parser.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either skip the top-level <url> capture when other top-level container sections (<organization>, <issueManagement>, <ciManagement>, <distributionManagement>, <parent>) are open, or only honor <url> when at column-0 inside <project>
- [ ] #2 Add a regression test with <organization><url>...</url></organization> followed by <scm><url>...</url></scm> asserting scm_url matches the SCM URL
<!-- AC:END -->
