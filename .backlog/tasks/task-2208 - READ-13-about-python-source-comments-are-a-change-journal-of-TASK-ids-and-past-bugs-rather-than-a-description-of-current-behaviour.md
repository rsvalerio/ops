---
id: TASK-2208
title: >-
  READ-13: about-python source comments are a change journal of TASK ids and
  past bugs rather than a description of current behaviour
status: To Do
assignee:
  - TASK-2248
created_date: '2026-09-08 07:20'
updated_date: '2026-09-08 11:01'
labels:
  - code-review-rust
  - readability
dependencies: []
modified_files:
  - extensions-python/about/src/lib.rs
  - extensions-python/about/src/units.rs
priority: low
ordinal: 120000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-python/about/src/lib.rs:12`, `extensions-python/about/src/lib.rs:140`, `extensions-python/about/src/lib.rs:383`, `extensions-python/about/src/units.rs:51`

**What**: A large share of the crate's prose narrates history rather than behaviour. Examples:

- `lib.rs:12-17` — a crate-level comment explaining which `clippy` allows *used to* sit there and why they were removed.
- `lib.rs:392-408` — three stacked paragraphs on `normalize_urls` describing the previous two-map shape, "the previous shape kept a second same-sized `first_seen_raw` map read on exactly one line", and which TASK changed it.
- `lib.rs:302-307`, `lib.rs:355-359`, `lib.rs:449-472` — "matching on `text: Some(_)` first let ...", "Previously `pick_url` built a fresh `Vec<...>` ...", a multi-paragraph SEC rationale essay inside a three-line function.
- `units.rs:87-91`, `units.rs:130-132` — same pattern.

Nearly every comment is anchored to a TASK id (TASK-0394, 0484, 0569, 0704, 0816, 0854, 0964, 0974, 0980, 0985, 0987, 0991, 1062, 1110, 1207, 1254, 1258, 1755-1774), which reads as a changelog embedded in the source.

**Why it matters**: A reader has to filter the current contract out of a history of shapes that no longer exist. The rationale that matters (drop-don't-strip for control chars, scheme allowlist, first-seen collision policy) is buried among migration notes, and each is duplicated at the call site and again in the test docs. Git history already holds the "why we changed it"; the source should state the invariant.

**Twins**: TASK-2155 (`cargo-update`) and TASK-2182 (`ops-deps`) file the same finding against other crates. Apply one consistent policy.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Comments state the current invariant and the reason it must hold, not the shape that preceded it
- [ ] #2 TASK-id references are kept only where they point at a still-relevant policy decision, not as changelog anchors on every block
- [ ] #3 The SEC/ERR rationale for a policy lives at one location and is referenced, not restated at the call site and again in the test doc
<!-- AC:END -->
