---
id: TASK-0695
title: 'API-9: AboutCard public struct lacks #[non_exhaustive]'
status: To Do
assignee: []
created_date: '2026-04-30 05:26'
updated_date: '2026-04-30 06:07'
labels:
  - code-review-rust
  - API
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/project_identity/card.rs:14-18`

**What**: `pub struct AboutCard { pub description: Option<String>, pub fields: Vec<(String, String)> }` is re-exported from `ops_core::project_identity::AboutCard` and consumed by extension/CLI code that renders it. Unlike its sibling `ProjectIdentity` (already `#[non_exhaustive]` per TASK-0167) and the output-layer `StepLine`/`ErrorDetail` (TASK-0454), `AboutCard` is constructable via struct-literal syntax from outside the crate, making any future field addition (e.g. badges, theme override) a breaking change.

**Why it matters**: `AboutCard` is on the extension-facing data path used by every stack-specific about provider for rendering; adding a field today requires a coordinated SemVer bump across all extensions instead of being additive.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Mark AboutCard with #[non_exhaustive]
- [ ] #2 Provide a constructor (e.g. AboutCard::new) for downstream use
- [ ] #3 Update any in-crate construction sites to go through the constructor or pub-field assignment
<!-- AC:END -->
