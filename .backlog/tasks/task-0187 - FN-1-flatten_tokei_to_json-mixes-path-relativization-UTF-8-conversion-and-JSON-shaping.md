---
id: TASK-0187
title: >-
  FN-1: flatten_tokei_to_json mixes path relativization, UTF-8 conversion, and
  JSON shaping
status: To Do
assignee: []
created_date: '2026-04-22 21:25'
updated_date: '2026-04-23 06:45'
labels:
  - rust-code-review
  - FN
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/tokei/src/lib.rs:124-148`

**What**: flatten_tokei_to_json is a nested double-loop (languages then reports) mixing path-relativization, UTF-8 lossy conversion, and serde_json record building in one block. Would read better split into helpers like report_to_json and language_records.

**Why it matters**: FN-1/READ-3 — one concern per function. The loop body contains three distinct concerns.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 flatten_tokei_to_json is split into smaller named helpers
- [ ] #2 Per-report transformation has its own unit test
<!-- AC:END -->
