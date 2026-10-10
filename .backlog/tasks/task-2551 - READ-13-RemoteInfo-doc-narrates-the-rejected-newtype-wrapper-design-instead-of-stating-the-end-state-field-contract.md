---
id: TASK-2551
title: 'READ-13: RemoteInfo doc narrates the rejected newtype-wrapper design instead of stating the end-state field contract'
status: Done
assignee: []
created_date: '2026-10-10 15:39'
updated_date: '2026-10-10 22:11'
labels:
  - code-review-rust
  - readability
dependencies: []
parent_task_id: 'TASK-2624'
modified_files:
  - extensions/git/src/remote.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions/git/src/remote.rs:RemoteInfo'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/git/src/remote.rs:5-14`

**What**: The `RemoteInfo` struct doc opens with a design-journey essay: "Newtype wrappers (`Host`, `Owner`, `RepoName`, `RepoUrl`) were considered for argument-order safety, but every consumer accesses fields by name (never positionally) and the JSON serialization shape would have to be hand-rolled to strip the wrapper — paying complexity for no caller-side win. Revisit if a function takes multiple of these as positional arguments." This is a "why we picked X over Y" record of alternatives weighed during a past change, plus a stale-able trigger for reopening the decision.

**Why it matters**: READ-13: documentation describes the end state, not the journey. The passage renders in `cargo doc` for a public struct yet is meaningless to a caller using the API — it names refactors that never landed and consumers that were surveyed at write time. The durable content is already stated by the rest of the doc (fields produced only by `parse_remote_url`, serialized flat into JSON, `url` invariant). A reader who joined after the decision would delete the wrappers paragraph verbatim; it belongs in the PR description or an ADR. Same cleanup was already applied in sibling crates (TASK-2136 for run-before-commit).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 RemoteInfo doc states the enduring contract (bare String fields, constructed only by parse_remote_url, consumed by name, serialized flat) in one sentence without the considered-alternatives essay or the 'Revisit if...' trigger
- [x] #2 No behavior change; cargo test -p ops-git still passes

<!-- AC:END -->
