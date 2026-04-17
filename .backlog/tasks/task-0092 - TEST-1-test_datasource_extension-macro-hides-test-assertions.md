---
id: TASK-0092
title: 'TEST-1: test_datasource_extension! macro hides test assertions'
status: To Do
assignee: []
created_date: '2026-04-17 11:33'
updated_date: '2026-04-17 12:07'
labels:
  - rust-codereview
  - test
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/tokei/src/tests.rs:9`

**What**: test_datasource_extension! generates tests whose coverage is hidden behind the macro; review cannot verify field-level checks.

**Why it matters**: Hidden macro-generated assertions make it difficult to audit what is really tested.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Document the set of assertions in the macro or expand key ones inline
- [ ] #2 Add a direct test that asserts TokeiExtension.extension_types() == DATASOURCE
<!-- AC:END -->
