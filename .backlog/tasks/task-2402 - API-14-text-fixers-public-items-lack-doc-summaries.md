---
id: TASK-2402
title: 'API-14: text-fixers public items lack doc summaries'
status: Triage
assignee: []
created_date: '2026-10-04 14:16'
labels:
  - code-review-rust
  - API
dependencies: []
modified_files:
  - extensions/text-fixers/src/eof.rs
  - extensions/text-fixers/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'API-14:extensions/text-fixers/src/eof.rs:fix_eof'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/text-fixers/src/eof.rs:26` (`fix_eof`), `extensions/text-fixers/src/lib.rs:69-73` (`NAME`, `DESCRIPTION`, `SHORTNAME`, `TextFixersExtension`)

**What**: `pub fn fix_eof` has only `#[must_use]` and no `///` summary (its sibling `fix_trailing` is documented); the crate-root pub consts `NAME`/`DESCRIPTION`/`SHORTNAME` and `pub struct TextFixersExtension` have no doc comments.

**Why it matters**: API-14 requires a summary on every public item; `fix_eof` is the main public entry of the eof module and its None-means-clean contract is only discoverable by reading the body.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 fix_eof has a short summary documenting the Option return (None = already correct) and the CRLF/lone-CR rules
- [ ] #2 NAME, DESCRIPTION, SHORTNAME and TextFixersExtension carry doc summaries
<!-- AC:END -->
