---
id: TASK-1777
title: >-
  READ-3: trim_nonempty is imported 360 lines into the file, between two
  function definitions
status: To Do
assignee:
  - TASK-1992
created_date: '2026-08-27 11:22'
updated_date: '2026-08-28 14:11'
labels:
  - code-review-rust
  - readability
dependencies: []
modified_files:
  - extensions-python/about/src/lib.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-python/about/src/lib.rs:385-388`

**What**: A `use` statement sits in the middle of the module body, after every production function and immediately before `#[cfg(test)] mod tests`:

```rust
/// DUP-3 / TASK-1258: route through the shared
/// [`ops_about::text_util::trim_nonempty`] so the about-python and
/// about-node ERR-2 contracts are pinned at the same source location.
use ops_about::text_util::trim_nonempty;
```

Every other import in the crate is in the header block at `lib.rs:24-29`. `trim_nonempty` is used from four call sites spread across the file (`lib.rs:211-214`, `236-239`, `252-253`, `367`), all of them *above* this line, so a reader tracing where the symbol comes from has to scroll past the whole module to find it.

The rationale comment is worth keeping — it just belongs on the import in the header block, or as a note on the call sites.

**Why it matters**: the file already opens with a documented import block; a stray `use` 360 lines down breaks the one place a reader expects to resolve unqualified names, and it reads as an accidental leftover from the TASK-1258 refactor rather than a deliberate choice. It also invites the next edit to add another mid-file import, since precedent now exists.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The trim_nonempty import moves into the header import block at the top of lib.rs
- [ ] #2 The DUP-3 / TASK-1258 rationale comment is preserved alongside it
- [ ] #3 No use statement remains in the module body between function definitions
<!-- AC:END -->
