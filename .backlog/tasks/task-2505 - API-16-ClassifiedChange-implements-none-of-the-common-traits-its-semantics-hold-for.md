---
id: TASK-2505
title: 'API-16: ClassifiedChange implements none of the common traits its semantics hold for'
status: To Do
assignee: []
created_date: '2026-10-10 15:31'
updated_date: '2026-10-10 21:12'
labels:
  - code-review-rust
  - api
dependencies: []
parent_task_id: 'TASK-2622'
modified_files:
  - extensions-terraform/plan/src/model.rs
priority: low
ordinal: 1000
dedup_key: 'API-16:extensions-terraform/plan/src/model.rs:ClassifiedChange'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/plan/src/model.rs:177`

**What**: ClassifiedChange — the crate's central public output type, handed back by parse_and_classify and sliced through by has_changes — carries no derives at all: no Debug, no Clone, while every neighbouring public type has them (Action: Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord; PlanOptions: Debug, Clone; Plan/ResourceChange/Change: Debug). The semantics trivially hold: every field is Action, String, Option<String> or Vec<String>, all of them sanitized display data with no secret-bearing surface, so the usual caution (SEC-5/SEC-21) does not apply.

**Why it matters**: API-16 / C-COMMON-TRAITS — common traits are cheap now and impossible for a consumer to add later, because the fields are pub but the type lives in this crate. Without Debug a library caller cannot log, dbg!, or assert_eq! on a misclassified plan; without Clone they cannot keep a copy across a move into a renderer. Adding derives later is a semver-visible change for no benefit.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 ClassifiedChange derives at least Debug and Clone (PartialEq/Eq may follow if equality on display data is deemed meaningful)
- [ ] #2 cargo check -p ops-tfplan and cargo test -p ops-tfplan pass with the derives added
<!-- AC:END -->
