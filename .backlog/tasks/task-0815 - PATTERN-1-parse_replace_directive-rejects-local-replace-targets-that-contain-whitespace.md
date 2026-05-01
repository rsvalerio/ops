---
id: TASK-0815
title: >-
  PATTERN-1: parse_replace_directive rejects local replace targets that contain
  whitespace
status: Done
assignee:
  - TASK-0823
created_date: '2026-05-01 06:03'
updated_date: '2026-05-01 09:21'
labels:
  - code-review-rust
  - patterns
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-go/about/src/go_mod.rs:65-87`

**What**: if target.split_whitespace().count() > 1 returns None — correctly drops remote replace mod vX.Y.Z, but also drops ./local path/sub (a Go-mod-legal filesystem path with a space) before the path-prefix check.

**Why it matters**: Go cmd/go accepts paths with spaces; the heuristic is over-eager. Diverges from the documented intent (any filesystem path). Mirrors TASK-0686 (rejected absolute/parent-relative).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Only reject if the second whitespace-separated token looks like a vX.Y.Z version
- [ ] #2 Test for replace ex.com/m =-arrow ./has space/sub
<!-- AC:END -->
