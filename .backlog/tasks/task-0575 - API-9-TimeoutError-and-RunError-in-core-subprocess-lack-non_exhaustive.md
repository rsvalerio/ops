---
id: TASK-0575
title: 'API-9: TimeoutError and RunError in core::subprocess lack #[non_exhaustive]'
status: Triage
assignee: []
created_date: '2026-04-29 05:16'
labels:
  - code-review-rust
  - API
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/subprocess.rs:69`

**What**: `pub struct TimeoutError` (line 69) has two pub fields and `pub enum RunError { Io, Timeout }` (line 90) has two variants; neither is `#[non_exhaustive]`. These are the types the cargo-runner subprocess helper hands back to every Rust extension callsite.

**Why it matters**: API-9. Adding RunError::Killed(SignalKind) or TimeoutError.attempts requires a major bump as written.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 TimeoutError and RunError annotated #[non_exhaustive]
- [ ] #2 Existing matches inside the crate compile
<!-- AC:END -->
