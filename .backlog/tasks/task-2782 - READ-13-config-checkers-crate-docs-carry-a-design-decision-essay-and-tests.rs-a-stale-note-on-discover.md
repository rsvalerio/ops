---
id: TASK-2782
title: 'READ-13: config-checkers crate docs carry a design-decision essay and tests.rs a stale note on discover'
status: Done
assignee: []
created_date: '2026-11-18 18:44'
updated_date: '2026-11-18 21:37'
labels:
  - code-review-rust
  - readability
dependencies: []
parent_task_id: 'TASK-2859'
modified_files:
  - extensions/config-checkers/src/lib.rs
  - extensions/config-checkers/src/tests.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions/config-checkers/src/lib.rs:crate-docs'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/config-checkers/src/lib.rs:10-22` (crate `//!` docs, "Dependency note"); `extensions/config-checkers/src/tests.rs:378-381`

**What**:
- The crate-level docs end in a "Dependency note" that argues why discovery stays in `ops_text_fixers` rather than `ops-core` ("If a third extension ever needs it, extraction to its own crate is the move; two consumers do not justify it"). That is a design-decision record, not an enduring property a reader of the API needs.
- `tests.rs:378-381` says "`discovery::discover` currently swallows filesystem errors and returns an empty list, so the `with_context` wrap ... is a defensive future-proof". That is no longer true: `discover` returns `io::Result<Discovery>`, git failures other than not-a-repository/absent propagate as `Err`, and walk errors are surfaced through `Discovery::walk_errors`.

**Why it matters**: Narrative and "currently" notes go stale while looking authoritative; the tests.rs one already has, and it discourages writing the missing test for the discovery-failure context path.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The lib.rs Dependency note is reduced to the enduring fact (discovery is shared with ops_text_fixers) or moved to an ADR/PR description
- [x] #2 The stale tests.rs note about discover swallowing errors is removed or corrected
- [x] #3 If a discover Err is reachable from a test (e.g. tracked_only against a root git cannot enter), a test asserts the error carries the check-json label and root context

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.23 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
