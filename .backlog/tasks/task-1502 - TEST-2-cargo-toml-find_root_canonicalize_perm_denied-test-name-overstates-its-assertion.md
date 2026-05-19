---
id: TASK-1502
title: >-
  TEST-2: cargo-toml find_root_canonicalize_perm_denied test name overstates its
  assertion
status: To Do
assignee:
  - TASK-1573
created_date: '2026-05-18 18:04'
updated_date: '2026-05-19 16:45'
labels:
  - code-review-rust
  - tests
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/tests/find_root.rs:344-372`

**What**: Test name `find_root_canonicalize_perm_denied_keeps_canonicalize_failed_variant` promises "keeps CanonicalizeFailed variant", but the assertion accepts `CanonicalizeFailed { .. } | NotFound { .. }` because Linux/macOS surface EACCES differently. The body comment acknowledges this drift.

**Why it matters**: TEST-2 requires the test name to encode the expected outcome; here the name advertises a stronger contract than the test actually pins, so a regression that silently flips from CanonicalizeFailed to NotFound (or vice-versa) on a single platform will pass without surfacing.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either rename to e.g. find_root_canonicalize_perm_denied_returns_typed_error (matching what is actually asserted), or
- [ ] #2 Split into two #[cfg(target_os = ...)]-gated tests that each pin one variant
<!-- AC:END -->
