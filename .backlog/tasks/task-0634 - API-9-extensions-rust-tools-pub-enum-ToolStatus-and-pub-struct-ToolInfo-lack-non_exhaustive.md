---
id: TASK-0634
title: >-
  API-9: extensions-rust tools pub enum ToolStatus and pub struct ToolInfo lack
  #[non_exhaustive]
status: Done
assignee:
  - TASK-0636
created_date: '2026-04-29 05:50'
updated_date: '2026-04-29 06:17'
labels:
  - code-review-rust
  - api-design
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/tools/src/lib.rs:48` (ToolStatus), `extensions-rust/tools/src/lib.rs:56` (ToolInfo)

**What**: `pub enum ToolStatus { Installed, NotInstalled, Unknown }` and `pub struct ToolInfo { name, description, status, has_rustup_component }` are both reachable from the public `collect_tools()` API but neither carries `#[non_exhaustive]`. ToolInfo also exposes all four fields as `pub`, allowing exhaustive struct construction by downstream callers.

**Why it matters**: API-9 — adding a new ToolStatus variant (e.g. `Outdated`) or a new ToolInfo field (e.g. `version`) is a SemVer break for any extension or downstream crate doing exhaustive matching or struct-literal construction. The convention in the rest of this workspace is to mark public extension types `#[non_exhaustive]` (see TASK-0628 et al.).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 ToolStatus is annotated with #[non_exhaustive]
- [ ] #2 ToolInfo is annotated with #[non_exhaustive]
- [ ] #3 Internal exhaustive matches/literals updated to use _/Default as needed; cargo build --all-targets passes
<!-- AC:END -->
