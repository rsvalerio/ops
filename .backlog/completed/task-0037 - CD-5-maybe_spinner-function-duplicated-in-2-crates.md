---
id: TASK-0037
title: 'CD-5: maybe_spinner() function duplicated in 2 crates'
status: Done
assignee: []
created_date: '2026-04-14 20:11'
updated_date: '2026-04-15 09:56'
labels:
  - rust-code-duplication
  - DUP-1
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Anchor**: fn maybe_spinner
**Crate(s)**: extensions/about, extensions-rust/about
**Rule**: DUP-1 (5+ identical lines)

Identical 10-line function in:
- extensions/about/src/lib.rs:13
- extensions-rust/about/src/query.rs:56

Both create an indicatif spinner with the same template ("{spinner:.cyan} {msg}"), tick chars (braille pattern), and 80ms tick interval. Pure copy-paste.

**Fix**: Extract to a shared UI utility (e.g. ops_core::style or a new ops_ui crate), or into the extension crate's shared types.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 maybe_spinner defined in exactly one location and imported by both crates
<!-- AC:END -->
