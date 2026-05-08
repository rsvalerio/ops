---
id: TASK-1124
title: 'TEST-25: name_from_module_path test exercises only std rsplit, not crate logic'
status: To Do
assignee:
  - TASK-1266
created_date: '2026-05-08 07:27'
updated_date: '2026-05-08 13:19'
labels:
  - code-review-rust
  - tests
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-go/about/src/lib.rs:160-165`

**What**: The test `name_from_module_path` asserts `"github.com/openbao/openbao".rsplit('/').next().unwrap() == "openbao"` — it is a unit test of the standard library's `rsplit` method. It does not exercise `GoIdentityProvider`, `parse_go_mod`, `compute_module_count`, or any function defined in the `about-go` crate.

**Why it matters**: TEST-25 flags framework-only tests that exercise external libraries without testing crate logic. This adds noise to the failure surface — if `rsplit` ever changed semantics, this test would fail with no actionable signal — and gives a false sense of coverage for the actual `name = go_mod.module.rsplit('/').next()` site in `provide`.

**Suggested fix**: Delete the test; the existing `provide_simple_module_name` test already covers the `name` projection through the real provider entry point.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Standard-library-only test removed
- [ ] #2 Coverage of name projection is preserved through provider-level tests
<!-- AC:END -->
