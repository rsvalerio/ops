---
id: TASK-0748
title: 'ARCH-9: ConfigurableTheme(pub ThemeConfig) exposes inner config publicly'
status: Done
assignee:
  - TASK-0828
created_date: '2026-05-01 05:52'
updated_date: '2026-05-02 07:47'
labels:
  - code-review-rust
  - api
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/theme/src/configurable.rs:28`

**What**: ConfigurableTheme is a tuple struct with single pub field. Any caller holding &mut ConfigurableTheme can mutate underlying ThemeConfig post-construction, bypassing any future invariants.

**Why it matters**: ARCH-9: minimal public surface. Forecloses on adding construction-time invariants without a SemVer break.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Field becomes private (or pub(crate)) with ConfigurableTheme::new(config: ThemeConfig) constructor and read accessors
- [ ] #2 All in-tree callers go through the constructor
- [ ] #3 Document as SemVer-breaking adjustment if the type is part of a published surface; otherwise gate behind #[non_exhaustive]
<!-- AC:END -->
