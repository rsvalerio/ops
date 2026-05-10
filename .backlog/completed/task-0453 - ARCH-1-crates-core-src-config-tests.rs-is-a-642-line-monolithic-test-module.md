---
id: TASK-0453
title: 'ARCH-1: crates/core/src/config/tests.rs is a 642-line monolithic test module'
status: Done
assignee:
  - TASK-0538
created_date: '2026-04-28 05:44'
updated_date: '2026-04-28 13:32'
labels:
  - code-review-rust
  - ARCH
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/tests.rs:1-642`

**What**: Single 642-line test module covering merge_config, init_template, validation, alias resolution, and serde round-trips together. Same monolithic-test-module pattern flagged for the runner crate (TASK-0423).

**Why it matters**: Tests are a navigational hazard at this size; failures attribute to a giant module rather than the feature under test.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Split per-feature: merge_tests.rs, template_tests.rs, validate_tests.rs, serde_tests.rs
- [x] #2 All tests still pass; no helpers duplicated
<!-- AC:END -->
