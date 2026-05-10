---
id: TASK-0568
title: >-
  READ-2: go.work block opener requires exact spaced 'use (' — no-space variant
  silently disables block mode
status: Done
assignee:
  - TASK-0641
created_date: '2026-04-29 05:04'
updated_date: '2026-04-29 12:05'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-go/about/src/go_work.rs:25`

**What**: The block-form trigger compares the line equal to the literal use-space-paren. If a go.work file is written with use immediately followed by paren (no space, accepted by gofmt-style variants and cmd/go) the parser stays at top-level and falls through to strip_prefix of use-space, which then fails to match. Block-form members are silently skipped.

**Why it matters**: Hand-written or formatter-variant go.work files yield empty workspace listings without diagnostics.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Block opener accepts optional whitespace between use and opening paren
- [ ] #2 Same fix applied to the replace opener in extensions-go/about/src/go_mod.rs:51
<!-- AC:END -->
