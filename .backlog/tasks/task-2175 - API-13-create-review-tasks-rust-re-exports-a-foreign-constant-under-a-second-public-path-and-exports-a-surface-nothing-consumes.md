---
id: TASK-2175
title: >-
  API-13: create-review-tasks-rust re-exports a foreign constant under a second
  public path and exports a surface nothing consumes
status: To Do
assignee:
  - TASK-2247
created_date: '2026-09-08 07:12'
updated_date: '2026-09-08 11:00'
labels:
  - code-review-rust
  - api-design
dependencies: []
modified_files:
  - extensions-rust/create-review-tasks/src/lib.rs
  - extensions-rust/create-review-tasks/src/provider.rs
priority: low
ordinal: 88000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/create-review-tasks/src/lib.rs:17-22`

**What**: `pub const DATA_PROVIDER_NAME: &str = ops_create_review_tasks::DATA_PROVIDER_NAME;` publishes another crate's constant under a second public path. `ops_create_review_tasks::DATA_PROVIDER_NAME` remains public and is the canonical name — `provider.rs:37` reads it directly rather than going through the alias — so the same value is now reachable at two public paths, and a consumer has no way to tell which is authoritative.

Beyond that, none of the five public items has a consumer outside this crate. `crates/cli/src/main.rs:33` links the crate with a bare `extern crate ops_create_review_tasks_rust;` purely so the `linkme` factory slice is populated; no path in the workspace names `NAME`, `DESCRIPTION`, `SHORTNAME`, `DATA_PROVIDER_NAME`, or `CreateReviewTasksRustExtension`. The `impl_extension!` macro takes them as expressions and does not require them to be `pub` (cf. `extensions/about/src/lib.rs:51` and `extensions-rust/metadata/src/lib.rs:52`, which keep `NAME` private).

**Why it matters**: API-13. The re-export duplicates a still-public path — the same shape already filed as TASK-2072 (`ops-about`), TASK-2112 (`ops-git`) and TASK-2128 (hook config). Every `pub` here is public API this crate must not break, bought for no caller.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The DATA_PROVIDER_NAME alias is removed, or justified in its doc comment as the crate's single canonical path with ops_create_review_tasks::DATA_PROVIDER_NAME no longer read directly from provider.rs
- [ ] #2 Public items with no cross-crate consumer are narrowed to pub(crate) or private, matching the pattern in extensions/about and extensions-rust/metadata
- [ ] #3 cargo build -p ops-cli --features stack-rust still links the extension and the review_targets provider is still registered (existing registration test passes)
<!-- AC:END -->
