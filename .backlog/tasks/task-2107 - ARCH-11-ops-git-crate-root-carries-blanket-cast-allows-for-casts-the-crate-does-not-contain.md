---
id: TASK-2107
title: 'ARCH-11: ops-git crate root carries blanket cast allows for casts the crate does not contain'
status: To Do
assignee: []
created_date: '2026-09-08 06:53'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-rust
  - architecture
dependencies: []
parent_task_id: 'TASK-2246'
modified_files:
  - extensions/git/src/lib.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/git/src/lib.rs:9-17`

**What**: The crate root has

```rust
#![cfg_attr(test, allow(
    clippy::unwrap_used,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
))]
```

There is not a single `as` cast anywhere in `extensions/git/src` (grep for ` as u`/` as i`/` as f` returns nothing outside a prose comment), and the workspace already denies `as_conversions` globally. Three of the four allows are dead.

**Why it matters**: AGENTS.md and `docs/clippy.md` state that lint levels are centralized in `[workspace.lints]` and that an exception must be granted at the narrowest scope that works, with the reason written next to it. A crate-root blanket allow for lints that never fire is exactly the "policy problem masking a code problem" the workspace lint comment warns about: it silently pre-authorizes future lossy casts in this crate's tests with no reviewer signal.

<!-- scan confidence: verified — no cast expressions exist in this crate -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The three cast_* allows are removed from the ops-git crate root
- [ ] #2 cargo clippy --all-targets -p ops-git -- -D warnings still passes
- [ ] #3 The remaining clippy::unwrap_used test allow keeps (or gains) a one-line reason comment
<!-- AC:END -->
