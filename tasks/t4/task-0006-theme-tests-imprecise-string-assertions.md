---
id: TASK-006
title: "Theme tests rely on imprecise string contains() assertions"
status: Triage
assignee: []
created_date: '2026-04-08 00:00:00'
labels: [rust-test-quality, TQ, TEST-11, low, crate-theme]
dependencies: []
---

## Description

**Location**: `crates/theme/src/tests.rs`
**Anchor**: `mod tests`
**Impact**: 60+ tests use `.contains()` for output validation rather than asserting precise rendered output. While this makes tests resilient to formatting changes, it also means a rendering regression could pass if the expected substring still appears somewhere in the output.

**Notes**:
- TEST-11: Assert specific values, not just containment
- This is a trade-off: precise assertions are brittle to formatting changes, but `.contains()` can mask bugs
- Severity is low because the tests are numerous and cover diverse scenarios, providing good aggregate confidence
- Consider adding a few snapshot tests (e.g., `insta` crate) for key rendering scenarios to complement the `.contains()` approach
