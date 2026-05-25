---
id: TASK-1280
title: 'TEST-25: register_extension_data_providers_empty_inputs has no assertion'
status: Done
assignee:
  - TASK-1304
created_date: '2026-05-11 15:25'
updated_date: '2026-05-11 18:06'
labels:
  - code-review-rust
  - test
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/registry/tests.rs:565-573`

**What**: The test registers no extensions into a fresh `DataRegistry` and finishes with `let _ = registry;`. The body comment acknowledges `DataRegistry` doesn't expose `len/is_empty` so the test only checks that the call doesn't panic — there is no positive assertion of behaviour.

**Why it matters**: A test whose only failure mode is panic-on-call cannot detect any silent regression (e.g. a future change that registers a hidden default provider). Either add a `get` round-trip that proves no entries exist, or expose `is_empty()` on `DataRegistry` and assert on it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Add a concrete assertion — e.g. assert!(registry.get("any_name").is_none()) for a representative name, or DataRegistry::is_empty() if exposed
- [ ] #2 If DataRegistry cannot be inspected at all, mark the test #[ignore = "reason"] rather than keeping a no-op
<!-- AC:END -->
