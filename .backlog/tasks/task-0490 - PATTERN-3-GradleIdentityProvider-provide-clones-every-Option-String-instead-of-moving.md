---
id: TASK-0490
title: >-
  PATTERN-3: GradleIdentityProvider::provide clones every Option<String> instead
  of moving
status: Done
assignee:
  - TASK-0531
created_date: '2026-04-28 06:09'
updated_date: '2026-04-28 07:25'
labels:
  - code-review-rust
  - pattern
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-java/about/src/gradle.rs:29-35`

**What**: provide() takes settings/props/build by value but uses `.as_ref().and_then(|s| s.X.clone())` for each Option<String> field, allocating an extra String for name/version/description even though the source structs are dropped right after.

**Why it matters**: Three avoidable String allocations per provide() call, and the .as_ref().and_then(...).clone() pattern is heavier reading than destructuring. Same anti-pattern that TASK-0395 fixed for the Pyproject parser.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Destructure GradleSettings/GradleProperties/GradleBuild so name/version/description move out without clones
- [ ] #2 No .clone() calls remain in GradleIdentityProvider::provide for Option<String> fields
- [ ] #3 Existing gradle_provider_provide_full / gradle_provider_provide_minimal tests still pass
<!-- AC:END -->
