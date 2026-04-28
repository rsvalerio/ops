---
id: TASK-0460
title: 'DUP-3: secret_patterns kept in sync only by a runtime invariant test'
status: To Do
assignee:
  - TASK-0538
created_date: '2026-04-28 05:45'
updated_date: '2026-04-28 07:14'
labels:
  - code-review-rust
  - DUP
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/secret_patterns.rs:16-40`

**What**: SENSITIVE_KEY_PATTERNS (10 entries) and SENSITIVE_REDACTION_PATTERNS (8 entries, strict subset) are two hand-maintained `&[&str]` arrays whose subset relation is enforced only by a `#[test]`. Adding a pattern to one without the other compiles cleanly and only fails the unit test.

**Why it matters**: Encoding the relationship structurally would make it a compile-time fact instead of a CI gate, eliminating 8 of 10 verbatim-duplicated entries.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 One source-of-truth array exists; warn-but-don't-redact entries kept in a separate small array and the warn list is computed by chaining
- [ ] #2 redaction_patterns_is_subset_of_key_patterns test becomes structurally redundant or trivially provable
<!-- AC:END -->
