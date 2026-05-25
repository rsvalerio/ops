---
id: TASK-1105
title: >-
  PATTERN-1: wrap_text silently discards trailing word when max_lines is reached
  mid-iteration
status: Done
assignee: []
created_date: '2026-05-07 21:34'
updated_date: '2026-05-08 06:51'
labels:
  - code-review-rust
  - pattern
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/text_util.rs:64-114` (specifically lines 87-94, 97-99)

**What**: When `current_line` is non-empty and a word does not fit, the function pushes `current_line`, resets it to the new word, and then breaks if `lines.len() >= max_lines`. The post-loop block (line 97) only pushes `current_line` when `lines.len() < max_lines`, so the just-added word is dropped silently — no ellipsis. The intermediate-line truncation pass only runs over already-emitted lines.

**Why it matters**: Description fields rendered through cards lose tail content without "..." indicator.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 When the final word would exceed max_lines, the last emitted line ends with \u{2026} or another truncation marker
- [x] #2 A regression test: wrap_text("alpha beta gamma delta", 5, 2) either returns ["alpha", "beta\u{2026}"] or pins the chosen truncation policy explicitly
- [x] #3 Document the chosen behaviour in the function's doc comment so callers know whether trailing content survives
<!-- AC:END -->
