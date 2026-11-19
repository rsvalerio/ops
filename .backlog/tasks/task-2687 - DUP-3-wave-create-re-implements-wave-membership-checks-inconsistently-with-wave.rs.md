---
id: TASK-2687
title: 'DUP-3: wave create re-implements wave/membership checks inconsistently with wave.rs'
status: Done
assignee: []
created_date: '2026-11-18 18:27'
updated_date: '2026-11-18 21:41'
labels:
  - code-review-rust
  - DUP
dependencies: []
parent_task_id: 'TASK-2838'
modified_files:
  - crates/backlog/src/cmd/wave_create.rs
  - crates/backlog/src/cmd/wave.rs
priority: medium
ordinal: 1000
dedup_key: 'DUP-3:crates/backlog/src/cmd/wave_create.rs:resolve_members'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/backlog/src/cmd/wave_create.rs:130-161` (`resolve_members`), `crates/backlog/src/cmd/wave_create.rs:57-113` (`run_wave_create`)

**What**: `resolve_members` hand-rolls the "is a wave" and "already in a wave" checks instead of reusing `wave::is_wave` / `wave::is_member`, and the copies disagree:
- `wave_create.rs:151` compares the marker case-sensitively (`l == marker`, `a == marker`) while `wave::is_wave` (`wave.rs:644-654`) uses `eq_ignore_ascii_case` — a wave labelled `Code-Review-Wave` is accepted as a member of another wave.
- "already a member" (`wave_create.rs:147`) only reads `parent_task_id`; a task that belongs to an open wave via that wave's `dependencies:` or the pre-migration assignee link (both honoured by `wave::is_member`) is accepted into a second wave.
- Members are matched with exact case-insensitive id equality (`wave_create.rs:140`), not `Store::find`'s bare/padded forms (`1764.02`), unlike every other id-taking command.
- The membership check runs on a `scan_tasks` snapshot with no lock, and members are then rewritten from that stale snapshot (`wave_create.rs:96-111`): two concurrent `wave create` calls can both claim the same member, and any edit made to a member in between is overwritten.

**Why it matters**: The invariant "a member belongs to one wave" that triage and `wave overlap` rely on is not actually enforced.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 resolve_members uses the shared wave predicates (is_wave and the membership union) so marker matching and membership rules are identical to wave list/members
- [x] #2 Member ids resolve with the same bare/padded forms Store::find accepts
- [x] #3 Members are re-read under a lock (or compare-before-write) so concurrent wave creates cannot both claim one member and member edits are not overwritten

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.01 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
