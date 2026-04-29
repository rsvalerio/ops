---
id: TASK-0572
title: 'API-9: ResolveExecError and ExpandError public enums lack #[non_exhaustive]'
status: Triage
assignee: []
created_date: '2026-04-29 05:16'
labels:
  - code-review-rust
  - API
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/mod.rs:48`

**What**: Both `pub enum ResolveExecError` (line 48) and `pub enum ExpandError` (line 59) are part of the runner public surface but neither carries `#[non_exhaustive]`. Adding a new failure variant is a SemVer break for any downstream matcher.

**Why it matters**: API-9: public enums that document themselves as the canonical typed failure must be `#[non_exhaustive]`. The crate already follows this for `RunnerEvent` and `ExtensionInfo`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 ResolveExecError annotated with #[non_exhaustive]
- [ ] #2 ExpandError annotated with #[non_exhaustive]
- [ ] #3 All in-crate match sites verified to compile
<!-- AC:END -->
