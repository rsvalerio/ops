---
id: TASK-0377
title: 'ERR-1: read_crate_metadata swallows TOML parse errors with no diagnostic'
status: Done
assignee:
  - TASK-0417
created_date: '2026-04-26 09:38'
updated_date: '2026-04-27 19:56'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/units.rs:80`

**What**: Both fs::read_to_string and toml::from_str errors are mapped to (None, None, None) with no logging. A malformed Cargo.toml in a workspace member silently produces a unit named Default::default() with empty version/description.

**Why it matters**: Hides legitimate misconfiguration from the user. The _ discards on Err(_) lose actionable context.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Log at tracing::debug! (or warn! for parse errors) with the path and underlying error
- [ ] #2 Tests verify a malformed Cargo.toml produces a logged event (or returns Result so the caller can decide)
<!-- AC:END -->
