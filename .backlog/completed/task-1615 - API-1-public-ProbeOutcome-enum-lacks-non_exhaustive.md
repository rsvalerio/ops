---
id: TASK-1615
title: 'API-1: public ProbeOutcome enum lacks #[non_exhaustive]'
status: Done
assignee:
  - TASK-1638
created_date: '2026-05-22 06:51'
updated_date: '2026-05-22 13:20'
labels:
  - code-review-rust
  - api-design
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/tools/src/probe/timeout.rs:24-28` (re-exported as `crate::ProbeOutcome` from `lib.rs:21`)

**What**: `pub enum ProbeOutcome<T> { Ok(T), Failed }` is re-exported from the crate root but is not marked `#[non_exhaustive]`. Sibling `ToolStatus` in `lib.rs:73-83` already carries the attribute and explains the policy in its docstring (`API / TASK-1200`). The pattern in `lib.rs:178-198` already foreshadows a third variant (timeout vs spawn-IO distinction) that the wrapper in `timeout.rs:60-83` collapses today but may want to preserve later.

**Why it matters**: Adding a new variant later becomes a breaking change for any downstream `match` outside the crate. Since `ProbeOutcome` is part of the contract that `check_cargo_tool_installed`, `capture_cargo_list`, `check_rustup_component_installed`, and `capture_rustup_components` all expose, the freedom to extend it should be preserved up front to match `ToolStatus`'s policy.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Add #[non_exhaustive] to ProbeOutcome and update any in-crate matches that become non-exhaustive (the wrapper in run_probe_with_timeout_inner and the four capture/check sites).
- [x] #2 Mirror the ToolStatus docstring rationale on ProbeOutcome so the policy is discoverable next time a variant is added.
<!-- AC:END -->
