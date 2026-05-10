---
id: TASK-0350
title: 'SEC-31: DataRegistry::register silently overwrites duplicate provider names'
status: Done
assignee:
  - TASK-0420
created_date: '2026-04-26 09:35'
updated_date: '2026-04-27 11:34'
labels:
  - code-review-rust
  - security
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/extension/src/data.rs:135`

**What**: DataRegistry::register calls self.providers.insert(name.into(), provider) and discards the returned Option. If two extensions register the same name, the second silently wins with no diagnostic. CommandRegistry has the same property.

**Why it matters**: Fail-open behaviour: collisions can swap out trusted built-in providers (identity, metadata) for whatever loads later, changing about-card field declarations or computed JSON.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 register returns Result<(), DuplicateProvider> (or panics in debug + tracing-warns in release) when name is already present
- [ ] #2 Add a test asserting that registering twice with the same name surfaces the collision
<!-- AC:END -->
