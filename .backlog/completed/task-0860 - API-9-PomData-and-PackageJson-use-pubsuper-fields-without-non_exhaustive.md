---
id: TASK-0860
title: 'API-9: PomData and PackageJson use pub(super) fields without #[non_exhaustive]'
status: Done
assignee: []
created_date: '2026-05-02 09:19'
updated_date: '2026-05-02 10:34'
labels:
  - code-review-rust
  - api-design
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-java/about/src/maven/pom.rs:28-41` and `extensions-node/about/src/package_json.rs:7-18`

**What**: PomData exposes every field pub(super) so maven/mod.rs can destructure. Adding a new POM field is a silent API change to consumers because pattern-matching with .. is fine but a reordered named-field consumer breaks. The struct should use non_exhaustive so future fields are additive at the type level.

**Why it matters**: Same family as the project-wide non_exhaustive audit. Internal data carriers benefit from the same guarantee - especially when shared across two files via pub(super).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 PomData and PackageJson gain non_exhaustive annotations or are deconstructed only via accessor methods
- [ ] #2 A test (or doc comment) confirms construction stays inside the parser module
- [ ] #3 Adding a new field does not require changes in the consuming mod.rs files
<!-- AC:END -->
