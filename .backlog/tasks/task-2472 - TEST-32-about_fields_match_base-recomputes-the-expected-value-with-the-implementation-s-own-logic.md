---
id: TASK-2472
title: 'TEST-32: about_fields_match_base recomputes the expected value with the implementation''s own logic'
status: Done
assignee: []
created_date: '2026-10-10 15:28'
updated_date: '2026-10-10 22:11'
labels:
  - code-review-rust
  - test
dependencies: []
parent_task_id: 'TASK-2621'
modified_files:
  - extensions-terraform/about/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'TEST-32:extensions-terraform/about/src/lib.rs:about_fields_match_base'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/about/src/lib.rs:1216`

**What**: `about_fields_match_base` asserts that `provider.about_fields()` matches `base_about_fields()` on length and zipped ids — but the implementation under test (`DataProvider::about_fields` for `TerraformIdentityProvider`, line 78-80) is literally `base_about_fields()`. Both sides of every assertion resolve through the same call, so the test is `X == X` by construction: it asserts ground truth back to itself and cannot fail for any reason anyone cares about, yet must be read and re-approved in every future diff.

**Why it matters**: TEST-32: a test that recomputes the expected value using the implementation's own logic verifies that the code equals itself. If the intent is to pin "the terraform provider exposes exactly the base about fields", the expected side should be independent — e.g. a pinned list of expected field ids, or a property the returned fields must satisfy (contains the identity/module/stack field ids the renderer requires). Same shape as TASK-2172 / TASK-2383 in sibling crates.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The test no longer derives its expected value from base_about_fields(), the same function the implementation delegates to
- [x] #2 The replacement asserts an independent expectation (pinned ids or a stated property) so a real regression in the provider's field set fails the test

<!-- AC:END -->
