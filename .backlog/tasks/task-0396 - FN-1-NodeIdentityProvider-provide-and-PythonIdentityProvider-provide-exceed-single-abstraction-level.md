---
id: TASK-0396
title: >-
  FN-1: NodeIdentityProvider::provide and PythonIdentityProvider::provide exceed
  single abstraction level
status: To Do
assignee:
  - TASK-0417
created_date: '2026-04-26 09:40'
updated_date: '2026-04-26 10:10'
labels:
  - code-review-rust
  - structure
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-node/about/src/lib.rs:55`

**What**: Both provide methods are ~40 lines and mix three abstraction levels: reading manifest, extracting individual fields with repeated parsed.as_ref().and_then(...), computing the composite stack_detail string, and constructing ProjectIdentity.

**Why it matters**: The repeated .and_then(|p| p.field.clone()) is exactly the pattern the shared-helper extraction (DUP-2) should remove.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extract fn build_stack_detail(engine_node: Option<&str>, pkg_manager: Option<&str>) -> Option<String> (Node) and equivalent for Python; cover with unit tests for all 4 None/Some combinations
- [ ] #2 After DUP-2 is addressed, provide becomes a 5-step orchestration with no inline .and_then patterns
<!-- AC:END -->
