---
id: TASK-2400
title: 'API-14: a few public ops-about items have no doc summary'
status: Done
assignee: []
created_date: '2026-10-04 14:15'
updated_date: '2026-10-04 15:54'
labels:
  - code-review-rust
  - API
dependencies: []
parent_task_id: 'TASK-2421'
modified_files:
  - extensions/about/src/lib.rs
  - extensions/about/src/lru.rs
  - extensions/about/src/text_util.rs
  - extensions/about/src/machine.rs
priority: low
ordinal: 1000
dedup_key: 'API-14:extensions/about/src:undocumented-pub-items'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/lib.rs:66` (`AboutExtension`), `extensions/about/src/lib.rs:110` (`AboutOptions::new`), `extensions/about/src/lru.rs:70` (`new`), `extensions/about/src/text_util.rs:109` (`pad_to_width_plain`); also the public fields of `AboutOptions` (`refresh`, `visible_fields`, `is_tty`) and `MachineReport` (`schema_version`, `kind`).

**What**: These public items lack the mandatory one-sentence doc summary.

**Why it matters**: Public API without summaries leaves rustdoc entries blank (API-14).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Every listed public item and field has a doc summary

<!-- AC:END -->
