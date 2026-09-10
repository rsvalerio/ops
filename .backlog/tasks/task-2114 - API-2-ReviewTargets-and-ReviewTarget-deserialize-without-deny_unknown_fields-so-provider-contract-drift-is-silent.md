---
id: TASK-2114
title: 'API-2: ReviewTargets and ReviewTarget deserialize without deny_unknown_fields, so provider contract drift is silent'
status: Done
assignee: []
created_date: '2026-09-08 06:53'
updated_date: '2026-09-10 15:53'
labels:
  - code-review-rust
  - api-design
dependencies: []
parent_task_id: 'TASK-2247'
modified_files:
  - extensions/create-review-tasks/src/lib.rs
priority: low
ordinal: 33000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/create-review-tasks/src/lib.rs:46` (`ReviewTargets`), `:57` (`ReviewTarget`)

**What**: Both payload types are plain `#[derive(Deserialize)]` with no `#[serde(deny_unknown_fields)]`. `fetch_review_targets` decodes whatever JSON the registered `review_targets` provider returns, and an extra or misspelled key is dropped without comment.

**Why it matters**: This struct is the whole contract between the generic engine and each stack-specific provider (`extensions-rust/create-review-tasks/src/provider.rs` today, others later). If a provider is updated to emit, say, `target` instead of `targets`, or adds a field the engine has not learned about yet, the mismatch surfaces either as a confusing "no targets" bail or as tasks quietly missing information — never as "your provider sent a key I do not understand". The crate is otherwise strict at exactly this boundary (`validate` rejects empty, over-long and control-character values with precise field paths), so the missing strictness here is inconsistent with its own design.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 ReviewTargets and ReviewTarget reject unknown fields at decode time
- [x] #2 A test asserts that a payload carrying an unexpected key fails with an error naming that key
- [x] #3 The existing provider in extensions-rust/create-review-tasks still decodes cleanly under the stricter contract

<!-- AC:END -->
