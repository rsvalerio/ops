---
id: TASK-2174
title: 'API-14: every public item in create-review-tasks-rust lacks a doc summary'
status: To Do
assignee: []
created_date: '2026-09-08 07:12'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-rust
  - api-design
dependencies: []
parent_task_id: 'TASK-2247'
modified_files:
  - extensions-rust/create-review-tasks/src/lib.rs
priority: medium
ordinal: 87000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/create-review-tasks/src/lib.rs:17-22`

**What**: The crate's entire public surface carries no doc comments:

- `pub const NAME` (:17)
- `pub const DESCRIPTION` (:18)
- `pub const SHORTNAME` (:19)
- `pub const DATA_PROVIDER_NAME` (:20)
- `pub struct CreateReviewTasksRustExtension` (:22)

The crate-level `//!` docs are present and good, but no individual public item has a summary line. `provider::SKILL_NAME`, `RustReviewTargetsProvider`, and the private helpers are documented — the gap is confined to lib.rs.

**Why it matters**: API-14. These are the only names a consumer of this crate sees; `rustdoc` renders five bare signatures with no explanation of which is the CLI-facing shortname, which is the extension identifier, and which is the engine's registry key. Same finding class as TASK-2138 / TASK-2149 / TASK-2163 on the other extension crates.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 NAME, DESCRIPTION, SHORTNAME, DATA_PROVIDER_NAME and CreateReviewTasksRustExtension each carry a doc summary stating what the item is and where it is consumed
- [ ] #2 Any item narrowed to pub(crate) or private under TASK-2175 is exempt; whatever remains public is documented
<!-- AC:END -->
