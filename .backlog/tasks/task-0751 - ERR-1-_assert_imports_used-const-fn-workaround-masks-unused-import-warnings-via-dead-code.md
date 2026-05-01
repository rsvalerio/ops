---
id: TASK-0751
title: >-
  ERR-1: _assert_imports_used const-fn workaround masks unused-import warnings
  via dead code
status: To Do
assignee:
  - TASK-0829
created_date: '2026-05-01 05:53'
updated_date: '2026-05-01 06:18'
labels:
  - code-review-rust
  - structure
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/extension/src/extension.rs:253-261`

**What**: A `const _: () = { fn _assert_imports_used() { let _ = std::any::type_name::<DataField>(); ... } };` block exists solely to suppress unused-import warnings for items the macros reference via $crate:: paths.

**Why it matters**: READ-4: explicit > implicit. Dead-code shim is a maintenance hazard — removing DataField from the module breaks the compile but no test explains why.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Remove unused use lines from extension.rs (or replace with pub use re-exports), and drop the _assert_imports_used shim
- [ ] #2 cargo check --all-features and cargo build --release produce no new warnings
<!-- AC:END -->
