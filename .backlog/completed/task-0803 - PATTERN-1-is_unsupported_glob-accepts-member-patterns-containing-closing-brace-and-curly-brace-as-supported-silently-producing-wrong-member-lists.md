---
id: TASK-0803
title: >-
  PATTERN-1: is_unsupported_glob accepts member patterns containing
  closing-brace and curly-brace as supported, silently producing wrong member
  lists
status: Done
assignee:
  - TASK-0822
created_date: '2026-05-01 06:01'
updated_date: '2026-05-01 07:00'
labels:
  - code-review-rust
  - patterns
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/query.rs:175-181`

**What**: is_unsupported_glob flags only ? and [ as unsupported, on top of the anything-after-first-* rule. Cargo glob shapes also include closing-class and brace-alternation in some glob libs. A pattern like crates/[a-z]* triggers the [ check, but crates/{core,cli} slips through entirely because none of *, ?, [ is present — read_dir then fails silently and the warn-log fires for the wrong reason.

**Why it matters**: Hand-rolled glob shape detection is fragile. Missing classes mean members are silently dropped.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either call into a vetted glob crate to decide expandability, or extend is_unsupported_glob to flag {, }, ] in addition to ?, [
- [ ] #2 Add unit tests for crates/{core,cli}, crates/[a-z]*, crates/foo? to prevent regression
- [ ] #3 Behaviour: any unsupported shape is passed through unchanged with the existing tracing::warn
<!-- AC:END -->
