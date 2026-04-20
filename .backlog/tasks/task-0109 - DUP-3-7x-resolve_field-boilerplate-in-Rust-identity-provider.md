---
id: TASK-0109
title: 'DUP-3: 7x resolve_field boilerplate in Rust identity provider'
status: Done
assignee: []
created_date: '2026-04-19 18:36'
updated_date: '2026-04-19 19:43'
labels:
  - rust-code-review
  - duplication
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/identity.rs:82-125`

**What**: `resolve_field(pkg, ws_pkg, |p| p.X.as_str(), |wp| wp.X.as_deref())` is repeated for 7 fields (version, description, edition, license, repository, homepage, msrv) with only the field accessor changing.

**Why it matters**: Near-identical 4-line blocks (DUP-3 — 3+ occurrences of the same pattern). Adding a new inheritable field requires another verbatim block; a data-driven table or macro would collapse these and make the inheritance semantics obvious.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Reduce repetition (macro, helper, or field-descriptor table) so adding a new field is a one-line change
- [x] #2 Existing test coverage for workspace inheritance still passes
<!-- AC:END -->
