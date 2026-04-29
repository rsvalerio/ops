---
id: TASK-0550
title: >-
  READ-5: wrap_text only truncates the final line, leaving overlong intermediate
  lines for unbreakable words
status: Triage
assignee: []
created_date: '2026-04-29 05:02'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/text_util.rs:49`

**What**: wrap_text allows a single word wider than max_width to land in current_line (it is pushed verbatim when current_line.is_empty()). The post-loop guard only truncate_to_widths lines.last_mut(), so any earlier line containing such a word is emitted at full width. With cards calling this at inner_width = 30, a 35-char URL or long identifier in description text breaks the card border.

**Why it matters**: The function contract (every line <= max_width) is silently violated for non-final intermediate lines.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every emitted line satisfies display_width(line) <= max_width, including intermediate lines
- [ ] #2 Regression test pins the contract for an unbreakable word in the first of three lines
<!-- AC:END -->
