---
id: TASK-2547
title: 'ARCH-17: ops-config-checkers crate created on edition 2021'
status: To Do
assignee: []
created_date: '2026-10-10 15:39'
updated_date: '2026-10-10 21:12'
labels:
  - code-review-rust
  - architecture
dependencies: []
parent_task_id: 'TASK-2608'
modified_files:
  - extensions/config-checkers/Cargo.toml
priority: medium
ordinal: 1000
dedup_key: 'ARCH-17:extensions/config-checkers/Cargo.toml:ops-config-checkers'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/config-checkers/Cargo.toml:5`

**What**: The crate inherits `edition.workspace = true`, and the workspace root pins `edition = "2021"`, so ops-config-checkers — created 2026-03 or earlier (present when the rust extensions moved to `extensions/` in 043e6b5c) — was created on edition 2021, not 2024. The crate sets no edition of its own; editions are per-crate, so it could carry `edition = "2024"` even inside the 2021 workspace.

**Why it matters**: An old edition on new code buys nothing and locks the crate out of let-chains (FN-2), `gen`, the `unsafe_op_in_unsafe_fn` default, and the 2024 match-ergonomics rules. Sibling crates were already filed for the same finding (TASK-2445 about-node, TASK-2476 about-java, TASK-2479 about-python, TASK-2475 about-go); this crate was missed by those waves.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 ops-config-checkers compiles with edition 2024 (crate-level edition = "2024", or a recorded workspace-wide migration decision)
- [ ] #2 cargo check -p ops-config-checkers and cargo test -p ops-config-checkers pass after the change

<!-- AC:END -->
