---
id: TASK-1363
title: >-
  TEST-25: update_toml_theme_theme_injection_prevention verifies round-trip but
  not the actual injection-defence contract
status: Done
assignee:
  - TASK-1384
created_date: '2026-05-12 21:29'
updated_date: '2026-05-12 23:23'
labels:
  - code-review-rust
  - tests
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/theme_cmd.rs:230`

**What**: The test's name claims it verifies injection prevention, but the body only checks `doc["output"]["theme"].as_str().unwrap() == malicious` — i.e. that the literal payload round-trips through TOML. A regression that pasted the raw string into TOML byte-for-byte (breaking neighbouring keys) would still satisfy the assertion.

**Why it matters**: TEST-25: test name advertises a security property the body doesn't actually pin. The injection-defence contract is "surrounding keys remain intact, document parses without errors, no new entries leak via the payload" — none of those are asserted.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Pre-populate the input with at least one sibling key after [output] and assert it survives unchanged after the rewrite
- [ ] #2 Assert the rewritten document parses without errors and that doc["output"] has only the expected keys (no payload-leaked entries)
<!-- AC:END -->
