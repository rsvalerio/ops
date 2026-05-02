---
id: TASK-0892
title: >-
  API-9: AboutCard::new positional constructor contradicts #[non_exhaustive]
  guarantee
status: Triage
assignee: []
created_date: '2026-05-02 09:46'
labels:
  - code-review-rust
  - api
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/project_identity/card.rs:96`

**What**: `AboutCard` is annotated `#[non_exhaustive]` (per TASK-0695) to permit future field additions without a breaking change. The new `AboutCard::new(description, fields)` constructor exposes every current field positionally. Adding a third field forces either a breaking signature change or a silent default — defeating the seal.

**Why it matters**: `non_exhaustive` plus a positional all-fields constructor is a contradictory API contract. Downstream callers that use `new` get zero benefit from `non_exhaustive` because every future field addition is still a breaking change for them.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace  with a builder (AboutCard::builder().description(..).fields(..).build()) or default-plus-setters
- [ ] #2 If only these two fields will ever exist, drop #[non_exhaustive]
- [ ] #3 Document the chosen evolution path in the rustdoc so the contradiction is intentional
<!-- AC:END -->
