---
id: TASK-0226
title: 'DUP-1: load_tokei duplicates TokeiIngestor::load path logic'
status: To Do
assignee: []
created_date: '2026-04-23 06:33'
updated_date: '2026-04-23 06:45'
labels:
  - rust-code-review
  - duplication
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/tokei/src/lib.rs:150`

**What**: `load_tokei` computes the same json_path check and calls ingestor.load — appears unused externally and overlaps with the DataIngestor impl.

**Why it matters**: Dead/duplicated code path can drift from the ingestor behavior.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Remove load_tokei or delegate fully to TokeiIngestor
- [ ] #2 Assert single ingestion entry point with a test
<!-- AC:END -->
