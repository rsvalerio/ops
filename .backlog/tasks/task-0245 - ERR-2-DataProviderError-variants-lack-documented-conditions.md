---
id: TASK-0245
title: 'ERR-2: DataProviderError variants lack documented conditions'
status: To Do
assignee: []
created_date: '2026-04-23 06:35'
updated_date: '2026-04-23 06:46'
labels:
  - rust-code-review
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/extension/src/error.rs:46`

**What**: The thiserror enum has no doc comments explaining when each variant is returned from public provide/get_or_provide.

**Why it matters**: Consumers cannot pattern-match on variants with confidence about what triggers each.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Add /// docs to each variant explaining the conditions
- [ ] #2 Cross-reference in DataProvider::provide docs
<!-- AC:END -->
