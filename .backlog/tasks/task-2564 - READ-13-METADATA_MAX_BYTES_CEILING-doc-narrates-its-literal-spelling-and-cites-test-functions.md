---
id: TASK-2564
title: 'READ-13: METADATA_MAX_BYTES_CEILING doc narrates its literal spelling and cites test functions'
status: To Do
assignee: []
created_date: '2026-10-10 15:41'
updated_date: '2026-10-10 21:12'
labels:
  - code-review-rust
  - readability
dependencies: []
parent_task_id: 'TASK-2619'
modified_files:
  - extensions-rust/metadata/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions-rust/metadata/src/lib.rs:METADATA_MAX_BYTES_CEILING'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/lib.rs:125-143` (esp. 137, 139-142)

**What**: The const's doc comment contains two process artifacts: (1) "(see `above_ceiling_warns_and_clamps` in `tests/payload_cap.rs`)" and "the equality with `u32::MAX` is pinned by `ceiling_is_exactly_u32_max` in `tests/payload_cap.rs`" — production docs citing test function names; (2) "Spelled as a literal because `u64::from` is not callable in a `const` initialiser and `u32::MAX as u64` would need an `as_conversions` exception (`docs/clippy.md`)" — narration of how the value happens to be spelled.

**Why it matters**: READ-13 — documentation describes the end state, not the journey that produced it. References from a production doc into test function names go stale the moment a test is renamed or moved while still looking authoritative, and the spelling rationale is a note to the diff reviewer, not to the API reader. The ceiling's *policy* content (a value above 4 GiB is almost certainly a typo or a guard-disable attempt; an unbounded knob would silently disable the cap) is enduring and should stay.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Doc keeps the ceiling policy (why 4 GiB, what an unbounded knob would do) and drops the test-function citations and the literal-spelling rationale
<!-- AC:END -->
