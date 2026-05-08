---
id: TASK-1245
title: >-
  PATTERN-1: read_gitdir_pointer matches first raw gitdir: line, no comment skip
  or whitespace trim
status: To Do
assignee:
  - TASK-1270
created_date: '2026-05-08 13:00'
updated_date: '2026-05-08 13:19'
labels:
  - code-review-rust
  - patterns
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/hook-common/src/git.rs:85`

**What**: `content.lines().find_map(|l| l.strip_prefix("gitdir:"))` accepts the first line whose raw text starts with `gitdir:` — no leading-whitespace strip, no comment skip, no rejection if multiple `gitdir:` lines exist. An indented `\tgitdir: /real` is ignored, and a hand-edited pointer with `gitdir: /attacker\n# gitdir: /real` resolves to the attacker path.

**Why it matters**: Real-world worktree pointers are single-line and well-formed, so impact is bounded, but the helper is the hook installer's path-resolution oracle and the parser accepts shapes wider than the format git itself writes.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Trim leading whitespace before strip_prefix; reject content with multiple gitdir: lines
- [ ] #2 Tests for indented and multi-line shapes
- [ ] #3 Document the accepted single-line shape in the helper doc
<!-- AC:END -->
