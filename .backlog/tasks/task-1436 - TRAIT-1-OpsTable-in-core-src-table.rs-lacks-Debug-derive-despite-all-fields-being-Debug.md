---
id: TASK-1436
title: >-
  TRAIT-1: OpsTable in core/src/table.rs lacks Debug derive despite all fields
  being Debug
status: To Do
assignee:
  - TASK-1459
created_date: '2026-05-13 18:33'
updated_date: '2026-05-13 19:09'
labels:
  - code-review-rust
  - trait
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/table.rs:14`

**What**: `pub struct OpsTable { inner: Table, is_tty: bool }` declares only `Default` and `Display` impls and no `#[derive(Debug)]`. Both fields (`comfy_table::Table`, `bool`) implement `Debug`, so the derive is mechanical.

**Why it matters**: Rust API guideline C-DEBUG: public types should derive `Debug` so they can be placed inside larger `#[derive(Debug)]` structs in downstream crates (CLI command structs, theme adapters) without hand-rolling an `impl Debug` purely to satisfy the bound. The omission is asymmetric with the rest of ops-core's public types.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 OpsTable derives Debug
- [ ] #2 cargo build -p ops-core --all-features passes
- [ ] #3 Downstream crates can put OpsTable inside their own #[derive(Debug)] structs without manual impls
<!-- AC:END -->
