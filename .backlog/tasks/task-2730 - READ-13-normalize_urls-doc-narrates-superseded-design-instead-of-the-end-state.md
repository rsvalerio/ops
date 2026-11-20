---
id: TASK-2730
title: 'READ-13: normalize_urls doc narrates superseded design instead of the end state'
status: Done
assignee: []
created_date: '2026-11-18 18:35'
updated_date: '2026-11-18 22:24'
labels:
  - code-review-rust
  - READ
dependencies: []
parent_task_id: 'TASK-2848'
modified_files:
  - extensions-python/about/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions-python/about/src/lib.rs:normalize_urls'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-python/about/src/lib.rs:421-436` (`normalize_urls` doc comment)

**What**: The `///` doc on `normalize_urls` narrates design history rather than describing the function:
- `lib.rs:422-424`: "The normalisation runs once per About call rather than once per `pick_url` candidate set." (contrasts with a prior implementation)
- `lib.rs:427-431`: "Collecting into a map would keep an arbitrary winner ... so the map is walked explicitly" (why-we-picked-X-over-Y)
- `lib.rs:433-436`: "keeping the raw key beside its URL makes 'the collision warn always has a first key to name' structural rather than an invariant two parallel containers must maintain" (references a removed two-container design)

The enduring contract (keys normalised case-insensitively with `-` == space; on collision first-seen wins and a `tracing::warn!` names both raw keys and URLs; result maps normalised key -> `(raw_key, url)`) is buried among this narrative.

**Why it matters**: Journey text is meaningless to a later reader and goes stale on the next change while looking authoritative; it belongs in the PR/commit, not item docs.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The normalize_urls doc states only the current behaviour: normalisation rule, first-seen-wins collision policy with its warn, and the shape of the returned map
- [x] #2 No sentence in the doc contrasts the code with a previous or alternative implementation

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.10 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
