---
id: TASK-2440
title: 'code-review-plan-wave52'
status: Done
assignee: []
created_date: '2026-10-10 14:31'
updated_date: '2026-10-10 14:59'
labels:
  - code-review-wave
dependencies:
  - TASK-2430
  - TASK-2433
  - TASK-2434
modified_files:
  - extensions/sqlite/src/sql/ingest/dir.rs
  - extensions/about/src/manifest_io.rs
  - extensions/about/src/manifest_cache.rs
  - extensions-go/about/src/go_mod.rs
  - extensions-go/about/src/go_work.rs
  - extensions-node/about/src/units.rs
  - extensions-java/about/src/maven/pom.rs
  - extensions-terraform/about/src/lib.rs
  - extensions/text-fixers/src/atomic.rs
  - crates/core/src/text.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
code-review-plan-wave52
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->

<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
All three are symlink/SEC hardening: the ingest parent-directory symlink refusal, read_optional_text following a symlinked root manifest out of the workspace, and the path-based (TOCTOU-windowed) write-back checks in text-fixers. Shared root cause: path-name-based checks where a handle-based one is needed.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Overlaps: none

Branch: code-review/TASK-2440
Worktree: /home/rsvalerio/projects/.wave-TASK-2440

<!-- SECTION:NOTES:END -->
