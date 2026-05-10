---
id: TASK-0412
title: >-
  ERR-1: parse_action_line silently returns None for unrecognised cargo update
  line shapes
status: Done
assignee:
  - TASK-0421
created_date: '2026-04-26 09:53'
updated_date: '2026-04-27 16:16'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-update/src/lib.rs:179-216`

**What**: `parse_action_line` returns `None` whenever:
- an `Updating` line lacks the `->` arrow form or has fewer than 4 whitespace-split parts, or
- an `Adding`/`Removing` line has fewer than 2 parts.

The outer loop in `parse_update_output` (lib.rs:111-114) drops `None` results without any tracing or counter. There is no warning when cargo-update output deviates from the expected shape (e.g. lines that include the registry suffix `(registry+...)`, multi-word notes, or new verbs in future cargo versions).

**Why it matters**: A cargo upgrade or a non-default registry can shift the shape of the output. Users will see "no updates available" when in fact the parser dropped every entry — same failure mode as TASK-0317 and TASK-0404, but on a different parser.

**Suggested**: emit `tracing::debug!(line = %clean, "skipping cargo update line that did not match any known verb shape")` for both the `parse_action_line == None` case and short-`Updating` early returns. Optionally surface the count via `CargoUpdateResult`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 parse_action_line / parse_update_output emit a debug trace when a candidate action line is dropped
- [ ] #2 Test asserts the trace fires for an Updating line that omits the arrow form
<!-- AC:END -->
