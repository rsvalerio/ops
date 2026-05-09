---
id: TASK-1252
title: >-
  ERR-1: is_index_progress_line rejects 2-token Updating <registry> forms
  causing false drift warns
status: Done
assignee:
  - TASK-1267
created_date: '2026-05-08 13:01'
updated_date: '2026-05-09 14:51'
labels:
  - code-review-rust
  - errors
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-update/src/lib.rs:259`

**What**: `is_index_progress_line` requires exactly three whitespace tokens with the third equal to `index`. A 2-token "Updating crates.io" returns false, falls through `parse_action_line` failure, then `starts_with_known_verb` fires the "possible format drift" warn — exactly the false-positive PATTERN-1 / TASK-1054 was meant to suppress.

**Why it matters**: A future or localised cargo-update progress line could flood operator logs with bogus drift warnings on every `ops about --refresh`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Relax the third-token check to allow end-of-line on a 2-token form (pin observed cargo behaviour in a comment)
- [x] #2 Extend starts_with_known_verb to require a v\d version token
- [x] #3 Unit test for the 2-token Updating crates.io no-warn case
<!-- AC:END -->
