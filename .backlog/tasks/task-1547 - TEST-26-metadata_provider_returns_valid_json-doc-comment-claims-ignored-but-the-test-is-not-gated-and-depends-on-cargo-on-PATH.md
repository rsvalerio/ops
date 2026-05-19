---
id: TASK-1547
title: >-
  TEST-26: metadata_provider_returns_valid_json doc-comment claims ignored but
  the test is not gated and depends on cargo on PATH
status: To Do
assignee:
  - TASK-1576
created_date: '2026-05-19 15:26'
updated_date: '2026-05-19 16:46'
labels:
  - code-review-rust
  - TEST
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/tests.rs:93-115`

**What**: The doc comment immediately above `metadata_provider_returns_valid_json` reads (line 95):

> This test is ignored because it requires:
> 1. cargo to be available in PATH
> 2. The test to run in a valid Cargo workspace
>
> Re-enable criteria: Run with cargo test -- --ignored when cargo is available

But the test is NOT marked `#[ignore]` — only `#[test]` (line 105). So the comment and the attribute disagree: the test runs in every default `cargo test` invocation and silently depends on `cargo` being on `PATH` and on `CARGO_MANIFEST_DIR` pointing at a valid workspace (which it does, by virtue of being a Cargo build).

**Why it matters**: TEST-26 covers documentation that disagrees with the test attribute, and TEST-24 covers `#[ignore]` without explanation. Here it is the inverse drift — the *narrative* claims `#[ignore]` while the attribute does not. Either the test should be marked `#[ignore]` to match its documented contract (and re-enabled in a `cargo test -- --ignored` lane), or the doc comment should be rewritten to acknowledge that the test always runs and relies on the build environment having `cargo` available. The same comment block also references "TQ-003" which is not a rule ID in the current taxonomy.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either the test is marked #[ignore] with a matching --ignored CI lane, or its doc comment is rewritten to reflect that it runs by default and depends on cargo being on PATH
- [ ] #2 The dangling 'TQ-003' reference is removed or replaced with a current rule ID
<!-- AC:END -->
