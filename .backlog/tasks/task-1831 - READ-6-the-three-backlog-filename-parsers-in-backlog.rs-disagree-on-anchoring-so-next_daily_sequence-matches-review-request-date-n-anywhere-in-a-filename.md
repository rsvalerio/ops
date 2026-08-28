---
id: TASK-1831
title: >-
  READ-6: the three backlog-filename parsers in backlog.rs disagree on
  anchoring, so next_daily_sequence matches review-request-<date>-<n> anywhere
  in a filename
status: Done
assignee:
  - TASK-2005
created_date: '2026-08-27 15:21'
updated_date: '2026-08-28 15:52'
labels:
  - code-review-rust
  - readability-consistency
dependencies: []
modified_files:
  - extensions/create-review-tasks/src/backlog.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/create-review-tasks/src/backlog.rs:80-84`, `110-138`

**What**: This module parses backlog filenames in three places and each one uses a different matching discipline for the same class of problem:

- `leading_task_number` (line 134) **anchors**: `file_name.strip_prefix("task-")?` — the digits must start the name.
- `review_request_sequence` (line 81) **does not anchor**: `file_name.split_once(prefix)?` finds `review-request-<date>-` at *any* offset, then takes the digit run that follows.
- `conflicting_claim` (line 122-124) uses a **third** discipline: it splits on the first `" - "` and requires the slug to be *exactly* `<slugified title>.md`.

The unanchored middle one is the defect. `next_daily_sequence(root, "2026-08-27")` counts any task file whose name merely *contains* `review-request-2026-08-27-` followed by a digit. Concretely, an ordinary hand-written backlog task named `Fix review-request-2026-08-27-3 flakiness` lands on disk as `task-1900 - Fix-review-request-2026-08-27-3-flakiness.md`; `split_once` finds the prefix mid-slug, `rest` is `3-flakiness.md`, `take_while(is_ascii_digit)` yields `3`, and the day's sequence jumps to 4 even though only two review requests exist. Nothing requires the digit run to be the whole remaining stem either — `…-3-flakiness` is accepted exactly like `…-3.md`.

The existing test `daily_sequence_ignores_other_days` only pins that `…-08-2-9` does not match `…-08-20-`; it never tests a name where the prefix appears anywhere other than immediately after `" - "`, so the anchoring difference is invisible to the suite.

The inconsistency compounds: because `next_daily_sequence` substring-matches while `conflicting_claim` requires exact slug equality, a filename that *inflates* the sequence is not a filename that can ever be *reported* as a conflicting claim. The two functions are supposed to be reasoning about the same set of names and they are not.

**Why it matters**: A sequence inflated by an unrelated task silently skips `review-request-<date>-<n>` values, so the per-day numbering the operator reads off the main-task title stops being a count of that day's review requests — the thing the title exists to communicate. It is also the same divergence TASK-1829 (DUP-1) has to work around: merging the two allocator walks only gives one place to fix filename parsing if all three parsers agree on what a task filename looks like. The fix is one shared anchored parser: split the stem on the first `" - "`, then match the prefix against the *start* of the slug and require the digit run to end at the extension.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 review_request_sequence anchors the review-request-<date>- prefix at the start of the filename slug (after the ' - ' separator) instead of matching it at any offset
- [x] #2 the digit run must be the complete remainder of the stem: 'task-1900 - Fix-review-request-2026-08-27-3-flakiness.md' does not contribute to next_daily_sequence(.., '2026-08-27')
- [x] #3 leading_task_number, review_request_sequence and conflicting_claim's title check share one filename-splitting helper, so all three agree on what the task number and the slug of a filename are
- [x] #4 a regression test covers a non-review-request task whose title embeds a review-request-<date>-<n> string and asserts the day's sequence is unaffected
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
All three parsers now go through one `TaskFileName::parse` helper, which splits
`<id> - <slug>.md` into an optional `task-<n>` number and the slug. `next_ids`
(number + daily sequence) and `conflicting_claim`'s title check both consume it, so
they can no longer disagree about where the number ends and the slug begins.

`TaskFileName::review_request_sequence` is anchored at both ends: the
`review-request-<date>-` prefix must start the slug, and the digit run must be the
entire remainder. New test
`daily_sequence_ignores_a_review_request_id_embedded_in_a_title` pins
`task-1900 - Fix-review-request-2026-08-27-3-flakiness.md` as contributing nothing.
<!-- SECTION:NOTES:END -->
