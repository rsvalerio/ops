---
id: TASK-2452
title: 'READ-13: step_line_theme doc comments still carry TASK tags and prior-implementation narration'
status: Done
assignee: []
created_date: '2026-10-10 15:24'
updated_date: '2026-10-10 21:25'
labels:
  - code-review-rust
  - readability
dependencies: []
parent_task_id: 'TASK-2609'
modified_files:
  - crates/theme/src/step_line_theme.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:crates/theme/src/step_line_theme.rs:mod step_line_theme'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/theme/src/step_line_theme.rs:9`, `:19-30`, `:60-62`

**What**: Doc comments in this file narrate the change history rather than the end state:
- `:9` — `/// SEC-15 / TASK-0358: NaN, negative, and infinite inputs render as \"--\"...` — a guideline/task self-report tag prefixing a otherwise fine behavioural statement.
- `:19-30` — the comment block inside `format_duration` opens with `ERR-5 / TASK-0857: ... replaces the prior \`try_from(_ as i128)\` indirection whose intent ... was hidden in the cast chain` — migration narration of a superseded implementation.
- `:60-62` — `/// Number of steps in a terminal state so far (CL-3 / TASK-0771: this includes failed and skipped...)` — task provenance in a field doc.

**Why it matters**: READ-13 — docs describe the end state, not the journey; task tags and "replaces the prior X" essays are process artifacts that go stale while looking authoritative. The durable content (guards reject non-finite/negative; the casts are clamp-bounded and intended; `completed` includes failed and skipped) should stay, the provenance should go.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 No /// or // comment in the file references a TASK id or a superseded implementation; the behavioural facts they carried remain stated in end-state terms

<!-- AC:END -->
