---
id: TASK-1360
title: >-
  DUP-3: print_extension_details and print_provider_info open with an 8-line
  identical KIND-header preamble
status: Done
assignee:
  - TASK-1383
created_date: '2026-05-12 21:29'
updated_date: '2026-05-12 23:16'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/extension_cmd.rs:317` and `crates/cli/src/extension_cmd.rs:357`

**What**: Both helpers render `"<KIND>: {tty? cyan(name) : name}\n{description}\n"` with the same TTY-check ternary. ~8 lines of byte-identical preamble copy-pasted.

**Why it matters**: Any future change to the heading style (e.g. bold for the KIND label, an icon prefix, an alignment tweak) has to land in both places or the two object-detail views drift. A `write_object_header(w, kind, name, description, is_tty)` helper collapses both call sites.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extract a write_object_header helper used by both print_extension_details and print_provider_info
- [ ] #2 Rendered output for both TTY and plain paths is byte-identical to today; new helper has a focused unit test
<!-- AC:END -->
