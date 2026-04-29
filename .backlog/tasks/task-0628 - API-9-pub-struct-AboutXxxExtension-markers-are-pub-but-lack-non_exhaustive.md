---
id: TASK-0628
title: 'API-9: pub struct AboutXxxExtension markers are pub but lack #[non_exhaustive]'
status: Done
assignee:
  - TASK-0636
created_date: '2026-04-29 05:22'
updated_date: '2026-04-29 06:17'
labels:
  - code-review-rust
  - API
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-go/about/src/lib.rs:25`

**What**: All four about crates expose a unit pub struct as the extension marker — extensions-go/about/src/lib.rs:25 (AboutGoExtension), extensions-python/about/src/lib.rs:26, extensions-node/about/src/lib.rs:27, extensions-java/about/src/lib.rs:28 (AboutMavenExtension) and :52 (AboutGradleExtension). Constructed only inside impl_extension! factory and never user-built. Each is pub but none carries #[non_exhaustive].

**Why it matters**: TASK-0468 flagged the same shape in a different crate. Cheap form of future-proofing the public surface.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each AboutXxxExtension marker gains #[non_exhaustive] OR is downgraded to pub(crate) if no external consumer needs it
- [ ] #2 cargo build --workspace succeeds
<!-- AC:END -->
