---
id: TASK-2566
title: 'READ-13: StagedFile doc narrates its design against the terraform pipeline''s helper and a rejected wrapper shape'
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
  - extensions-rust/metadata/src/ingestor.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions-rust/metadata/src/ingestor.rs:StagedFile'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/ingestor.rs:340-350`

**What**: The `StagedFile` doc narrates the design's provenance and a rejected alternative: "Mirrors the terraform pipeline's `with_artifact_cleanup`, using `Drop` rather than a wrapper because `load`'s early exits are `?` rather than a single fallible expression." The following paragraph also re-argues why cleanup is unconditional.

**Why it matters**: READ-13 — documentation describes the end state, not the journey. The cross-crate mirroring note and the Drop-vs-wrapper comparison are design-journal content: meaningless to a reader using the type, and stale the moment either pipeline changes shape. What endures is the behavioral contract: the guard owns `metadata.json` for the whole of `load` and unlinks it on every exit path.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Doc states the ownership/unlink-on-drop contract without the terraform-pipeline mirroring note and the Drop-versus-wrapper comparison
<!-- AC:END -->
