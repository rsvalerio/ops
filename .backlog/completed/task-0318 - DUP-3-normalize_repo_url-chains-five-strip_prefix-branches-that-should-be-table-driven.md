---
id: TASK-0318
title: >-
  DUP-3: normalize_repo_url chains five strip_prefix branches that should be
  table-driven
status: Done
assignee:
  - TASK-0327
created_date: '2026-04-24 08:53'
updated_date: '2026-04-25 13:43'
labels:
  - rust-code-review
  - duplication
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: extensions-node/about/src/lib.rs:215-233

**What**: Repeated s.strip_prefix(x).map(format!) pattern for github:/gitlab:/bitbucket:/git+/git:// prefixes.

**Why it matters**: DUP-3 threshold; hostname registry is hard to extend and easy to get out of sync with other about extensions.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extract const HOST_PREFIXES slice; iterate and rewrite
- [ ] #2 Consider sharing with other about extensions if shapes align
<!-- AC:END -->
