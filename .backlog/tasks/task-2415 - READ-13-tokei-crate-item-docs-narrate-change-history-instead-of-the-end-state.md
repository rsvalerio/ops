---
id: TASK-2415
title: 'READ-13: tokei crate/item docs narrate change history instead of the end state'
status: To Do
assignee: []
created_date: '2026-10-04 14:19'
updated_date: '2026-10-04 14:52'
labels:
  - code-review-rust
  - READ
dependencies: []
parent_task_id: 'TASK-2420'
modified_files:
  - extensions/tokei/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions/tokei/src/lib.rs:crate-docs'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/tokei/src/lib.rs:1-20` (crate docs), `:~120` (`TOKEI_DEFAULT_EXCLUDED` doc: "An earlier revision handed this list to tokei's own walker..."), `:~370` (`flatten_tokei_records` doc: "The public `flatten_tokei_to_json` wrapper that used to sit in front of this..."), `:~215` (PERF-3 comment recounting `get_statistics` pre-change behaviour); minor garbled comment in `screen_entry` ("Classify before stat'ing nothing else")

**What**: `//!` and `///` blocks describe removed symbols, earlier revisions and migrations, with task IDs, rather than the current behaviour. A reader of the API gains nothing from "`load_tokei` was removed" or "wrapper that used to sit in front".

**Why it matters**: this text goes stale and belongs in commit/PR history. The enduring invariants (exact-name, root-child-only pruning; why classification is repeated) should stay, stated as present-tense facts.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Docs state current behaviour only; references to removed items/earlier revisions are dropped or moved to the commit history
- [ ] #2 The garbled screen_entry comment is reworded
<!-- AC:END -->
