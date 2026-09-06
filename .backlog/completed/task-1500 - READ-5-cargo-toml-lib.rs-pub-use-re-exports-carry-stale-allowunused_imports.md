---
id: TASK-1500
title: >-
  READ-5: cargo-toml lib.rs pub use re-exports carry stale
  #[allow(unused_imports)]
status: Done
assignee:
  - TASK-1641
created_date: '2026-05-18 18:04'
updated_date: '2026-05-25 16:13'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/lib.rs:63-69`

**What**: Both `pub use inheritance::InheritanceError;` and the multi-item `pub use types::{...}` are annotated `#[allow(unused_imports)]`. `pub use` is never "unused" — it is part of the public API surface. The allow is either residue from when the items were private re-exports, or it silences a different lint by accident.

**Why it matters**: Stale `#[allow]` attributes erode the signal value of the warning system; a future engineer adding a genuine unused import next to these will not notice the new lint.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Both #[allow(unused_imports)] attributes removed from lib.rs pub use blocks
- [ ] #2 cargo build and cargo clippy --all-targets -- -D warnings remain clean
<!-- AC:END -->
