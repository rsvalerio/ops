---
id: TASK-2756
title: 'API-14: load_coverage IngestDir rationale renders inside its # Errors section'
status: Done
assignee: []
created_date: '2026-11-18 18:41'
updated_date: '2026-11-18 22:46'
labels:
  - code-review-rust
  - API
dependencies: []
parent_task_id: 'TASK-2854'
modified_files:
  - extensions-rust/test-coverage/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'API-14:extensions-rust/test-coverage/src/lib.rs:load_coverage'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/test-coverage/src/lib.rs:96-113`

**What**: The doc comment on the public `load_coverage` places the paragraph "Takes the verified [`IngestDir`] anchor the ingestor trait takes, so this public entry point cannot re-introduce a by-name resolution..." (lines 110-112) *after* the `# Errors` heading. Rustdoc renders everything after a heading as part of that section, so the parameter rationale appears under "Errors" as if it described a failure condition.

**Why it matters**: `load_coverage` is one of the crate's few exported items. Canonical doc sections must hold only what their heading says; the misplaced paragraph makes the error contract harder to read and hides the parameter explanation where readers won't look for it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The IngestDir paragraph is moved above the # Errors heading (into the description prose)
- [x] #2 The # Errors section of load_coverage lists only error conditions

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.19 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
