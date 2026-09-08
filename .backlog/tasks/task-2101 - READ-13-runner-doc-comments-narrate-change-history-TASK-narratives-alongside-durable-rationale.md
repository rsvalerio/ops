---
id: TASK-2101
title: >-
  READ-13: runner doc comments narrate change history (TASK narratives)
  alongside durable rationale
status: To Do
assignee:
  - TASK-2248
created_date: '2026-09-08 06:40'
updated_date: '2026-09-08 11:01'
labels:
  - code-review-rust
  - readability
dependencies: []
modified_files:
  - crates/runner/src/command/build.rs
  - crates/runner/src/command/results.rs
  - crates/runner/src/command/mod.rs
  - crates/runner/src/display/render_config.rs
  - crates/runner/src/display/progress_state.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: <!-- scan confidence: candidates to inspect --> `crates/runner/src/command/build.rs:40-64,316-367,406-439`; `crates/runner/src/command/results.rs:101-157`; `crates/runner/src/command/mod.rs:143-197`; `crates/runner/src/display/render_config.rs:16-29`; `crates/runner/src/display/progress_state.rs:24-41`

**What**: The largest doc blocks in the crate mix two kinds of prose: durable invariant rationale (why the cache is Mutex-guarded, why Deny fails closed — legitimately READ-4 "why") and change-history narration ("TASK-1940 — two properties this policy now actually delivers, having previously been weaker than the text above implied", "TASK-1140 lifts the previous Deny-only gate", "The previous shape allocated two Vecs", "Decision (TASK-1923)" essays). The second kind is a process artifact: it is meaningless to a reader using the API, goes stale on the next change while looking authoritative, and grows every review wave.

**Why it matters**: READ-13 — docs describe the end state, not the journey. The task backlog already owns the migration history (each TASK-xxxx cited is a filed task); duplicating it into doc comments means every future reader re-reads a diff narrative to find the one sentence of contract. Scoped to the runner crate's worst offenders, not a blanket style change: keep the invariant/contract prose, drop the "previously / pre-fix / prior to TASK-nnnn" narration or move it to the referenced task's notes.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each listed doc block states the current contract and invariants without narrating what a previous version did; no 'previously/pre-fix/prior to TASK-nnnn' sentences remain in the listed blocks
- [ ] #2 Durable rationale (fail-closed policy, cache bounds, !Send marker purpose, retention formula) is preserved — only the change-history narration is removed
- [ ] #3 Cited task numbers remain traceable via the backlog tasks, not the doc comments
<!-- AC:END -->
