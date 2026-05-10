---
id: TASK-0594
title: >-
  ERR-4: read_origin_url_from accepts the first url= line, contradicting
  git-config last-wins semantics
status: Done
assignee:
  - TASK-0639
created_date: '2026-04-29 05:18'
updated_date: '2026-04-29 10:58'
labels:
  - code-review-rust
  - ERR
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/git/src/config.rs:32`

**What**: read_origin_url_from returns the first url= line in [remote "origin"]. Real git-config: the last url value wins (git-config is multi-key; later assignments override). Returning the first means a config that overrides url later (templated includes do this) reports the obsolete URL, flowing into ProjectIdentity.repository.

**Why it matters**: ERR-4 — parser silently disagrees with git on key-resolution order. Doc comment notes Limitations re url.insteadOf but not first-vs-last.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 read_origin_url_from collects all url= matches in the section and returns the last
- [x] #2 Regression test exercises a config that sets url twice in [remote 'origin']
- [x] #3 Doc comment updated to note last-value-wins matches git-config semantics
<!-- AC:END -->
