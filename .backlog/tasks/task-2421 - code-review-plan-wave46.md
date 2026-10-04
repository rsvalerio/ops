---
id: TASK-2421
title: 'code-review-plan-wave46'
status: To Do
assignee: []
created_date: '2026-10-04 14:52'
updated_date: '2026-10-04 14:52'
labels:
  - code-review-wave
dependencies:
  - TASK-2394
  - TASK-2395
  - TASK-2396
  - TASK-2397
  - TASK-2398
  - TASK-2399
  - TASK-2400
modified_files:
  - extensions/about/src/lib.rs
  - extensions/about/src/lru.rs
  - extensions/about/src/machine.rs
  - extensions/about/src/manifest_cache.rs
  - extensions/about/src/manifest_io.rs
  - extensions/about/src/text_util.rs
  - extensions/about/src/workspace.rs
  - extensions/git/src/config.rs
  - extensions/git/src/lib.rs
  - extensions/git/src/provider.rs
  - extensions/git/src/remote.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
code-review-plan-wave46
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->

<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Generic about and git providers: symlink/FIFO hardening in workspace and manifest reads, bounded-read duplication in git config, docs.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Overlaps: none
<!-- SECTION:NOTES:END -->
