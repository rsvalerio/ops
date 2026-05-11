---
id: TASK-1301
title: >-
  TEST-11: register_extension_commands_aggregates_across_multiple_extensions
  silently passes (early-return) on the common <2-extension build
status: Done
assignee:
  - TASK-1304
created_date: '2026-05-11 16:36'
updated_date: '2026-05-11 18:07'
labels:
  - code-review-rust
  - tests
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/registry/tests.rs:576-600`

**What**: The test body opens with:
```rust
let exts = builtin_extensions(&config, std::path::Path::new(".")).unwrap();
if exts.len() < 2 {
    return;
}
```
The comment claims this skips meaningful assertion when fewer than 2 extensions are compiled in. In the default `cargo test -p ops-cli` invocation (no stack feature flags), `exts.len()` is typically 0 or 1 — so the test returns success without exercising the aggregation contract at all. The test name and the early-return are reading as a coverage win in CI dashboards but contribute zero signal on most builds.

**Why it matters**: Misleading green test. The "aggregation does not drop entries" contract this test claims to pin only fires on the subset of feature combinations that compile in ≥2 extensions; the rest of the matrix reports PASS while exercising nothing. Either inject two stub extensions inline (mirrors `register_extension_commands_detects_duplicate_command_id` which constructs ExtA + ExtB locally) so the test is feature-independent, or convert to `#[ignore]` with rationale and a feature-gated counterpart that asserts the real registry contract.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Test asserts aggregation behaviour using two stub Extensions defined inline (matching the duplicate-id test's pattern), so it never early-returns
- [ ] #2 OR: test is split into a feature-gated variant that exercises the real registry and a stub-driven variant that runs everywhere; the latter pins the contract
- [ ] #3 Running the test on a build with zero compiled-in extensions still produces an assertion (not a silent return Ok)
<!-- AC:END -->
