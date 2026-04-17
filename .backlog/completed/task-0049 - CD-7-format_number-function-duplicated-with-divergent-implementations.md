---
id: TASK-0049
title: 'CD-7: format_number function duplicated with divergent implementations'
status: Done
assignee: []
created_date: '2026-04-14 20:31'
updated_date: '2026-04-15 09:56'
labels:
  - rust-code-duplication
  - DUP-1
  - DUP-5
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Location**: `crates/core/src/project_identity.rs:249-259`, `extensions-rust/about/src/text_util.rs:4-17`
**Anchor**: `fn format_number`
**Impact**: Two independent implementations of comma-separated number formatting exist. The core version handles only positive i64 (uses `String::with_capacity`), while the extensions-rust version handles negative numbers via recursion (uses plain `String::new()`). Both share the same reverse-iterate-and-insert-comma algorithm. Having two copies risks behavioral drift — e.g., if negative-number support is needed in core, someone may not realize it already exists in text_util.

DUP-1: 10+ identical lines. DUP-5: extract shared logic into a single helper in `ops_core`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A single format_number lives in ops_core and both call sites use it
<!-- AC:END -->
