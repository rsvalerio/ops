---
id: TASK-2518
title: 'API-14: DataRegistry::new has no doc summary'
status: To Do
assignee: []
created_date: '2026-10-10 15:33'
updated_date: '2026-10-10 21:12'
labels:
  - code-review-rust
  - api
dependencies: []
parent_task_id: 'TASK-2612'
modified_files:
  - crates/extension/src/data.rs
priority: low
ordinal: 1000
dedup_key: 'API-14:crates/extension/src/data.rs:DataRegistry::new'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/extension/src/data.rs:231`

**What**: `DataRegistry::new` — the constructor every extension's `register_data_providers` starts from — has no doc comment, while its sibling `CommandRegistry::new` in `crates/extension/src/extension.rs:181` does ("Creates an empty registry with a cleared duplicate-insert audit trail").

**Why it matters**: API-14 — the summary sentence on a public item is mandatory. The asymmetry with the sibling constructor is the tell that this is an oversight, not policy; the interesting contract (starts empty, duplicate trail empty until `register` refuses a name) is exactly what the one-liner should say.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 DataRegistry::new carries a doc comment whose first paragraph is a ~15-word summary stating the registry starts empty with a cleared audit trail
<!-- AC:END -->
