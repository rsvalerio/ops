---
id: TASK-1433
title: >-
  READ-3: test_utils.rs (531 lines, test-support feature) lacks a module-level
  surface index
status: To Do
assignee:
  - TASK-1460
created_date: '2026-05-13 18:23'
updated_date: '2026-05-13 19:09'
labels:
  - code-review-rust
  - READ
  - API
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/test_utils.rs:1`

**What**: A 531-line test_utils module is compiled under the `test-support` feature and consumed across crates. There is no module-level rustdoc listing the public-under-feature surface vs. internal helpers.

**Why it matters**: Future feature additions may bind to helpers the core team treats as private; breaking changes are silent. A top-of-file index of what each public helper guarantees keeps the surface auditable.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Add a //! module-level doc enumerating the public test-support API and its stability contract
- [ ] #2 Internal-only helpers either gain pub(crate) visibility or move to a private submodule
<!-- AC:END -->
