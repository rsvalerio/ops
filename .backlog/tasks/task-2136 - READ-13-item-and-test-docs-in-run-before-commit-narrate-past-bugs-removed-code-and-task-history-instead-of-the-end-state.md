---
id: TASK-2136
title: >-
  READ-13: item and test docs in run-before-commit narrate past bugs, removed
  code and task history instead of the end state
status: To Do
assignee:
  - TASK-2248
created_date: '2026-09-08 06:56'
updated_date: '2026-09-08 11:01'
labels:
  - code-review-rust
  - readability
dependencies: []
modified_files:
  - extensions/run-before-commit/src/lib.rs
priority: low
ordinal: 52000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/run-before-commit/src/lib.rs:26`, `:89`, `:100`, `:126`, `:141`, `:358`, `:584`, `:620`

**What**: Doc comments across the crate document the journey rather than the API. Representative sites:

- `HOOK_SCRIPT` (:26-49) — a numbered essay on three "load-bearing properties" carrying four task IDs (`CL-3 / TASK-1910`, `ARCH-6 / TASK-1905`) and counterfactuals about what the hook used to do.
- `has_staged_files` (:100-111) — "SEC-31 / TASK-1903", explaining that the probe *used to* filter on `ACMR`.
- the tests-module comment (:126-131) — narrates a `#![cfg_attr(test, allow(...))]` block that no longer exists and why three of its four entries were wrong. Nothing in the file is described; only a deleted thing is.
- `retry_while_text_file_busy` (:358-371) and the large-output test (:584-592) — "TEST-15 / TASK-1913", "the old 1500 ms bound also raced the fake git's four forks".
- the section comment at :620-626 — "which no test touched before".

The enduring content in these blocks is real and worth keeping (the hook must not depend on bash; a staged deletion counts as staged work; the retry's worst case is 1 s). It is the surrounding migration narration — task IDs, "used to", "the old bound", descriptions of removed code — that a reader who joined after the decision would delete verbatim.

**Why it matters**: This is the crate's public documentation: `HOOK_SCRIPT`'s rationale and `has_staged_files`'s summary render in `cargo doc`, and the task IDs point at a backlog the reader cannot resolve. The narration also goes stale silently — the tests-module comment already describes a state of the file that no longer exists, while reading as authoritative.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Item docs state the enduring property (POSIX sh only, ops is probed before exec, --changed-only arms the preflight, every staged change kind counts, the retry's bounded worst case) without narrating what the code previously did
- [ ] #2 TASK-nnnn identifiers and rule IDs are removed from /// and //! blocks in this file, or moved to a comment inside the body where they annotate a specific line
- [ ] #3 The tests-module comment describing the removed cfg_attr allow block is deleted
- [ ] #4 cargo doc builds with no broken intra-doc links and the remaining docs still explain why each pinned property matters to a caller
<!-- AC:END -->
