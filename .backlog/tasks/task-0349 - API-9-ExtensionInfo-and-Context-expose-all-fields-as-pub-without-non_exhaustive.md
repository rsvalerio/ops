---
id: TASK-0349
title: >-
  API-9: ExtensionInfo and Context expose all fields as pub without
  #[non_exhaustive]
status: To Do
assignee:
  - TASK-0420
created_date: '2026-04-26 09:35'
updated_date: '2026-04-26 10:10'
labels:
  - code-review-rust
  - api
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/extension/src/extension.rs:36` and `crates/extension/src/data.rs:214`

**What**: ExtensionInfo is pub struct with every field pub and no #[non_exhaustive], unlike DataField/DataProviderSchema which are #[non_exhaustive]. Context has the same shape; data_cache is directly mutable by callers.

**Why it matters**: External extensions can construct ExtensionInfo via struct literals — adding a new field is a SemVer break. Direct pub access to data_cache lets callers bypass get_or_provide caching contract.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Mark ExtensionInfo #[non_exhaustive] and add a constructor; mark Context #[non_exhaustive] and gate data_cache behind accessor methods
- [ ] #2 Update internal call sites and tests to use the new constructors
<!-- AC:END -->
