---
id: TASK-0574
title: >-
  API-9: StepResult, CommandOutput, RenderConfig, DisplayOptions public structs
  lack #[non_exhaustive]
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
**File**: `crates/runner/src/command/results.rs:9`

**What**: StepResult (results.rs:9), CommandOutput (results.rs:75), RenderConfig (display/render_config.rs:10), DisplayOptions (display/render_config.rs:19) are all public structs with bare `pub` fields and no `#[non_exhaustive]`. Adding a field silently breaks downstream struct-literal constructors and pattern matches.

**Why it matters**: API-9. The crate has fixed this for events (RunnerEvent) and extension framework (ExtensionInfo, Context, DataField); these are stragglers.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 All four structs annotated with #[non_exhaustive]
- [ ] #2 Constructor (pub fn new) added where struct lacks one
- [ ] #3 In-crate construction sites compile cleanly
<!-- AC:END -->
