---
id: TASK-2809
title: 'READ-13: ops-about comments narrate past regressions and earlier implementations instead of the current contract'
status: To Do
assignee: []
created_date: '2026-11-18 18:46'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - READ
dependencies: []
parent_task_id: 'TASK-2858'
modified_files:
  - extensions/about/src/text_util.rs
  - extensions/about/src/identity.rs
  - extensions/about/src/cards.rs
  - extensions/about/src/deps.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions/about/src/text_util.rs:module-comments'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/text_util.rs:176-193` (comment on the `WIDTH_CHARS` counter: why counting beat a wall-clock ratio), `:238-242` (`wrap_text`: "the constant is ~5.6x on the shape ..."), `extensions/about/src/identity.rs:154-158` (`build_identity_value`: "the previous Display path embedded the same lossy bytes"), `extensions/about/src/cards.rs:192-198` (`row_parts` doc: "rather than inferred from a wall-clock ratio"), `:217-219` (`layout_cards_in_grid_with_width`: the same "avoid per-call allocation" comment written twice in a row), `extensions/about/src/deps.rs:346-350` (`format_dependencies_section`: hypothetical "future refactor" narration)

**What**: These comments describe how the code got here — earlier implementations, measured ratios from a past regression, why one test technique was preferred over another — rather than what the code guarantees now. The cards.rs pair at 217-219 is a leftover duplicate.

**Why it matters**: READ-13: journey narration goes stale after the next change while still reading as authoritative, and it buries the actual contract. Keep the invariant (e.g. "each word and each emitted line is measured once"), drop the history.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each listed comment states the current invariant only; historical/comparative narration is removed
- [ ] #2 The duplicated SPACING comment in cards.rs is reduced to one line
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.22 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
