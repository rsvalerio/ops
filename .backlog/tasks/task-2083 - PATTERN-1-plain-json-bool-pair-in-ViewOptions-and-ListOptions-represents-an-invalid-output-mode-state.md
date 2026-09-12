---
id: TASK-2083
title: 'PATTERN-1: plain/json bool pair in ViewOptions and ListOptions represents an invalid output-mode state'
status: Done
assignee: []
created_date: '2026-09-07 22:58'
updated_date: '2026-09-09 18:36'
labels:
  - code-review-rust
  - patterns
dependencies: []
parent_task_id: 'TASK-2243'
modified_files:
  - crates/backlog/src/cmd/view.rs
  - crates/backlog/src/cmd/list.rs
priority: low
ordinal: 11000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/backlog/src/cmd/view.rs:14`, `crates/backlog/src/cmd/list.rs:13`

**What**: `ViewOptions { plain: bool, json: bool }` and `ListOptions { plain: bool, json: bool }` can represent `(true, true)`, which is not a valid output mode. The ViewOptions doc comment admits it: "both set here is a caller bug rendered as plain". The same pair rides on the CLI boundary, where `--plain --json` together is exactly the kind of input the type should reject rather than silently resolve.

**Why it matters**: The invalid combination is documented as a caller bug instead of made unrepresentable — every future handler that copies the option struct inherits the ambiguity, and the resolution rule (plain wins) lives in prose per command. An `enum OutputFormat { Plain, Json }` field states the invariant once; `WaveListOptions`/`WaveMembersOptions` (json only) and `SearchOptions` (plain only) show the single-bool form is fine, so the finding is scoped to the two-bool structs.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 ViewOptions and ListOptions carry one output-mode field from which exactly one mode is always derivable (enum or equivalent), not two independent bools
- [x] #2 The both-set case is a compile-time or parse-time error rather than a documented fallback

<!-- AC:END -->
