---
id: TASK-0677
title: 'TRAIT-3: query_or_warn callers pass Default::default() with no explicit type'
status: To Do
assignee: []
created_date: '2026-04-30 05:14'
updated_date: '2026-04-30 06:07'
labels:
  - code-review-rust
  - traits
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/units.rs:46`, `extensions-rust/about/src/deps_provider.rs:29`, `extensions-rust/about/src/coverage_provider.rs:69`

**What**: Pass `Default::default()` as the fallback to `query_or_warn`. Type inference works today, but the value's type is fully implicit.

**Why it matters**: A future change to `query_or_warn`'s signature (or to the closure return type) will silently change which `Default` is selected — possibly the wrong empty container — and the warn-fallback path is rarely exercised in tests. Spelling the type (HashMap::new(), Vec::new()) makes the regression a compile error.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace Default::default() with the concrete empty value at each call site, or annotate the type explicitly
<!-- AC:END -->
