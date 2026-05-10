---
id: TASK-1196
title: >-
  API: PublishSpec::is_publishable returns true for unresolved Inherited
  workspace=true, inverting safe default
status: Done
assignee:
  - TASK-1269
created_date: '2026-05-08 08:14'
updated_date: '2026-05-10 16:23'
labels:
  - code-review-rust
  - api
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/types.rs:225-238`

**What**: `PublishSpec::is_publishable` matches `Inherited { .. } | None => true`. The doc-comment acknowledges this requires resolution to have run — but the type system gives callers no way to tell. A consumer that calls is_publishable() on a freshly parsed manifest gets true even when the workspace explicitly says publish = false. Any tool gating cargo publish on this method silently flips the safe-by-default direction.

**Why it matters**: A method whose documentation says "the answer is wrong unless the caller has run a separate function first" is a publish-side foot-gun. Either the unresolved variant should not implement is_publishable, or the resolution step should be encoded in the type (e.g. Resolved<PublishSpec> newtype, or Option<bool>).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 PublishSpec::is_publishable either returns Option<bool> (None for Inherited / pre-resolution) or is gated by a typestate so a caller cannot invoke it on an unresolved value.
- [x] #2 Existing call sites compile against the new signature and explicitly handle the unresolved case rather than silently treating it as publishable.
<!-- AC:END -->
