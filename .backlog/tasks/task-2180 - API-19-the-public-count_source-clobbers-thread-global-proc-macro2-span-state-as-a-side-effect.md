---
id: TASK-2180
title: >-
  API-19: the public count_source clobbers thread-global proc-macro2 span state
  as a side effect
status: To Do
assignee:
  - TASK-2246
created_date: '2026-09-08 07:13'
updated_date: '2026-09-08 10:59'
labels:
  - code-review-rust
  - api-design
dependencies: []
modified_files:
  - extensions-rust/loc/src/counter.rs
  - extensions-rust/loc/src/lib.rs
priority: low
ordinal: 93000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/loc/src/counter.rs:220` (`count_source`), also reachable via `count_fallback` callers

**What**: `counter::count_source` is `pub` in a `pub mod counter`, and its first statement is

```rust
proc_macro2::extra::invalidate_current_thread_spans();
```

That call drops proc-macro2's *thread-global* `span-locations` source map, invalidating every `Span` any code on that thread is holding — not just the ones this function is about to create. The rustdoc explains the memory rationale and asserts it "is safe here because spans never escape a single `count_source` call", which is true of this crate's own use but is not a property `count_source` can guarantee once it is public: any caller that lexes with `proc_macro2` and then calls `count_source` between creating a span and reading `.start()`/`.end()` gets silently wrong line numbers, with no error and no panic.

The retention problem is real and the mitigation is right; the issue is that a caller-visible global side effect is attached to a public function whose signature (`fn(&str, Region) -> FileCounts`) advertises a pure computation.

**Why it matters**: Global mutable state reached through an innocuous-looking public API is exactly the failure mode that is invisible at the call site and impossible to reproduce from the signature. Today the only caller is `count_entry` in the same crate, so nothing is broken — this is about not leaving the trap armed for the next caller.

Options, in preference order:
1. Make `count_source` (and `count_fallback`, `MAX_NESTING_DEPTH`) `pub(crate)`, so the invariant "spans never escape" is enforced by the module boundary rather than asserted in prose. Nothing outside the crate uses them.
2. If the entry point must stay public, move the invalidation to the crate-internal wrapper the walk calls and document the side effect in the *summary line* of the public one, not only in a `# Memory` section further down.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The function that calls invalidate_current_thread_spans is not reachable from outside the crate, or its public doc summary states the thread-global side effect in its first line
- [ ] #2 Items narrowed to pub(crate) are confirmed unused outside ops-rust-loc (grep the workspace) before narrowing
- [ ] #3 cargo check and the crate's test suite pass unchanged
<!-- AC:END -->
