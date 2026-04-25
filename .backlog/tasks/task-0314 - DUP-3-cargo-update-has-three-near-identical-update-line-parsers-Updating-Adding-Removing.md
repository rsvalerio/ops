---
id: TASK-0314
title: >-
  DUP-3: cargo-update has three near-identical update-line parsers
  (Updating/Adding/Removing)
status: Done
assignee:
  - TASK-0326
created_date: '2026-04-24 08:53'
updated_date: '2026-04-25 13:13'
labels:
  - rust-code-review
  - duplication
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: extensions-rust/cargo-update/src/lib.rs:162-207

**What**: parse_updating_line / parse_adding_line / parse_removing_line share structure and differ only in prefix and field mapping.

**Why it matters**: DUP-3 threshold — 3+ repeated patterns; future line-format changes need parallel edits.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Unify into a single parser driven by a (prefix, action, arity) table or mapping
- [ ] #2 Existing tests pass unchanged
<!-- AC:END -->
