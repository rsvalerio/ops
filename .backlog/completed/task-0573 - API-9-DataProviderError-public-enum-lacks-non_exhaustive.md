---
id: TASK-0573
title: 'API-9: DataProviderError public enum lacks #[non_exhaustive]'
status: Done
assignee:
  - TASK-0636
created_date: '2026-04-29 05:16'
updated_date: '2026-04-29 06:14'
labels:
  - code-review-rust
  - API
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/extension/src/error.rs:44`

**What**: `pub enum DataProviderError` is the canonical error returned by every extension `provide(...)` and matched by many downstream callers but is not `#[non_exhaustive]`. Adding a future variant is a SemVer break.

**Why it matters**: Same rationale as TASK-0468/0544/0545. Variants here grow naturally; locking them in by default breaks the contract that adding a variant is a minor-version bump.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 DataProviderError annotated with #[non_exhaustive]
- [ ] #2 All in-crate match sites compile (or get a wildcard arm)
<!-- AC:END -->
