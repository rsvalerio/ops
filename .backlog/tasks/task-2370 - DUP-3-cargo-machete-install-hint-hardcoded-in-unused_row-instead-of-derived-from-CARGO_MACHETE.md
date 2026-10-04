---
id: TASK-2370
title: 'DUP-3: cargo-machete install hint hardcoded in unused_row instead of derived from CARGO_MACHETE'
status: To Do
assignee: []
created_date: '2026-10-04 14:11'
updated_date: '2026-10-04 14:52'
labels:
  - code-review-rust
  - duplication
dependencies: []
parent_task_id: 'TASK-2417'
modified_files:
  - extensions-rust/deps/src/format.rs
  - extensions-rust/deps/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'DUP-3:extensions-rust/deps/src/format.rs:unused_row'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/format.rs:` `unused_row` (NotInstalled arm), duplicating `extensions-rust/deps/src/lib.rs` `CARGO_MACHETE.install_crate`

**What**: The skipped-row hint is the literal `"cargo-machete is not installed. Install with: cargo install cargo-machete"`, while the same crate/name already lives in `CARGO_MACHETE { subcommand, install_crate }`, which `external_tools()` and `check_tool_in` derive their messages from.

**Why it matters**: Two sources of truth for the install hint; renaming the crate or changing the probe updates one and not the other.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 unused_row builds its install hint from CARGO_MACHETE.install_crate (no second literal)
- [ ] #2 Existing render tests still pass
<!-- AC:END -->
