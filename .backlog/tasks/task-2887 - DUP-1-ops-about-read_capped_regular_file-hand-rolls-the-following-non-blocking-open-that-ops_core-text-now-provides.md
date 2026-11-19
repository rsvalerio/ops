---
id: TASK-2887
title: 'DUP-1: ops-about read_capped_regular_file hand-rolls the following non-blocking open that ops_core::text now provides'
status: Triage
assignee: []
created_date: '2026-11-19 08:08'
labels:
  - code-review-rust
  - duplication
dependencies: []
modified_files:
  - extensions/about/src/machine/layers.rs
  - extensions/about/Cargo.toml
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/machine/layers.rs:220` (`read_capped_regular_file`)

**What**: The helper builds its own `OpenOptions` with `custom_flags(libc::O_NONBLOCK)`, checks `is_file()` on the handle, and re-implements the `take(cap + 1)` byte cap. `ops_core::text::read_capped_bytes_following` (added alongside `open_regular_following`) now does exactly this: follows symlinks, opens `O_NONBLOCK`, type-checks the descriptor, clears `O_NONBLOCK`, and enforces the cap with a typed `CappedReadError`.

**Why it matters**: Two copies of the same hardened open will drift; the about copy also leaves `O_NONBLOCK` set on the returned handle and pulls in a direct `libc` dependency that would no longer be needed for it.

**Origin**: discovered during TASK-2881 while fixing TASK-2872.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 read_capped_regular_file reads through ops_core::text::read_capped_bytes_following (keeping its InvalidData/ cap error surface)
- [ ] #2 Existing FIFO and symlink-follow tests for the machine config layers still pass
<!-- AC:END -->
