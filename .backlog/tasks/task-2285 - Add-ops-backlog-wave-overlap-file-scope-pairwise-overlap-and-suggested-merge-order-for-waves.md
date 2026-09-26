---
id: TASK-2285
title: 'Add ops backlog wave overlap: file scope, pairwise overlap and suggested merge order for waves'
status: To Do
assignee: []
created_date: '2026-09-26 17:47'
updated_date: '2026-09-26 18:27'
labels:
  - feature
  - backlog
  - waves
  - skills-integration
dependencies: []
parent_task_id: 'TASK-2291'
modified_files:
  - crates/backlog/src/cmd/wave.rs
  - crates/backlog/src/render.rs
priority: medium
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**What**: `ops backlog wave overlap [<wave-id>…] [--json]`. For each wave it computes the union of its members' `modifiedFiles`, the pairwise overlap with every other open wave (the shared paths), and a suggested merge order, least-overlapping first.

**Why**: `code-review-triage` computes a wave's file scope and its overlap with every open wave by having the model read each task's `modifiedFiles` and do the set arithmetic. It then writes an `Overlaps: …` note by hand. This is deterministic data, and the model's arithmetic is not reliable across dozens of tasks. `code-review-run-waves` needs the same ordering.

**Optional follow-up in the same area**: `ops backlog wave create --members <ids>`, which creates the parent, sets `parent_task_id` and flips the members to To Do in one step. Today it takes one create and N edits.

**Used by**: code-review-triage, code-review-run-waves.
Source: skills-vs-ops audit of rsvalerio/ai dev-skills, 2026-09-26 (https://claude.ai/artifact/4yaKVkPdfe93hrqhFW5u1z).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 `wave overlap` reports each wave's file scope and the shared paths with every other open wave
- [ ] #2 It suggests a merge order, least-overlapping first, deterministic for ties
- [ ] #3 JSON output (`--json`) carries a schemaVersion
<!-- AC:END -->
