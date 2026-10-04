---
id: TASK-2359
title: 'FN-1: parse_package_json exceeds 50 lines and mixes read, deserialise and normalise'
status: To Do
assignee: []
created_date: '2026-10-04 14:10'
updated_date: '2026-10-04 14:52'
labels:
  - code-review-rust
  - fn
dependencies: []
parent_task_id: 'TASK-2424'
modified_files:
  - extensions-node/about/src/package_json.rs
priority: low
ordinal: 1000
dedup_key: 'FN-1:extensions-node/about/src/package_json.rs:parse_package_json'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-node/about/src/package_json.rs:85-165` (`parse_package_json`)

**What**: The function is ~80 lines and does four jobs at different abstraction levels: cache read, serde deserialise with failure reporting, author/contributor list assembly (`with_capacity` + two push loops), and per-field normalisation including the nested repository-object closure (`Object { url, directory }`, nesting 4-5 levels deep).

**Why it matters**: FN-1/FN-2. Hard to test the author assembly or repository projection in isolation; only whole-file tests reach them.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 parse_package_json is <= 50 lines, with author assembly and repository normalisation extracted into named helpers (e.g. collect_authors, normalise_repository)
- [ ] #2 Existing package_json and lib tests pass unchanged
<!-- AC:END -->
