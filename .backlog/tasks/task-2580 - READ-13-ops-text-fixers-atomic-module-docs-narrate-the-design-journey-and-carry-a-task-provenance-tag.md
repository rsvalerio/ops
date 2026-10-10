---
id: TASK-2580
title: 'READ-13: ops-text-fixers atomic module docs narrate the design journey and carry a task-provenance tag'
status: To Do
assignee: []
created_date: '2026-10-10 15:45'
updated_date: '2026-10-10 21:12'
labels:
  - code-review-rust
  - read
dependencies: []
parent_task_id: 'TASK-2627'
modified_files:
  - extensions/text-fixers/src/atomic.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions/text-fixers/src/atomic.rs:mod atomic'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/text-fixers/src/atomic.rs:3-70` (module docs), `extensions/text-fixers/src/atomic.rs:303` (test doc)

**What**: The `atomic` module docs narrate how the change was designed rather than what the module guarantees. The "# Why not `fs::write`" essay (lines 3-11) and "# The trade this makes" (lines 47-70, closing with "All three are accepted ... rare where interrupted hook runs are not") are "why we picked X over Y" process artifacts; the decision is already recorded in `.backlog/decisions/0001-text-fixers-write-back-residual-toctou-window.md`, which the docs themselves cite, so the rationale exists in two places and will drift. Line 303 carries a bare `(TASK-2434)` provenance tag in a test doc comment.

**Why it matters**: READ-13 — documentation describes the end state, not the journey that produced it. Design-journey narration goes stale on the next change while looking authoritative, and task tags are meaningless to a reader using the API. The durable safety contract (temp file in the same directory, fsync, rename(2), identity re-check, stage-prefix residue policy) is user-relevant and should stay; the comparative essay and the accepted-trade argumentation belong in the ADR that already holds them. This matches the cleanup already applied to the theme, about, and sqlite crates (recent commits stripping task tags and history narration from docs and tests).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Module docs state the enduring guarantees (atomic rename, attribute preservation, refusal conditions, stage-file residue behavior) without the 'why not fs::write' comparison essay or the 'all three are accepted' trade narration; design rationale lives only in the decision record
- [ ] #2 The (TASK-2434) tag at src/atomic.rs:303 is removed; the technical content of the comment (ctime is kernel-maintained) is kept
<!-- AC:END -->
