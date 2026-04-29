---
id: TASK-0587
title: 'API-9: pub struct ParsedManifest lacks #[non_exhaustive]'
status: Triage
assignee: []
created_date: '2026-04-29 05:18'
labels:
  - code-review-rust
  - API
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/identity.rs:19`

**What**: ParsedManifest is a pub struct with 17 public fields, designed for stack-specific identity providers (Rust, Go, Node, Python) to fill out and pass to build_identity_value. Stack providers struct-init it. Adding any new field is a breaking change for every out-of-crate stack extension.

**Why it matters**: API-9 — exactly the kind of growth-prone struct non_exhaustive exists for. Repo policy applied consistently elsewhere (TASK-0234/0260/0468/0546).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 ParsedManifest annotated #[non_exhaustive]
- [ ] #2 Existing in-crate struct-init uses ..Default::default()
- [ ] #3 Doc comment notes stable construction shape
<!-- AC:END -->
