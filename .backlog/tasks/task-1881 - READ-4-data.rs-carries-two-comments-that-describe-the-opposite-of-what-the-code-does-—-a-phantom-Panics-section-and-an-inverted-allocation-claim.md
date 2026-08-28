---
id: TASK-1881
title: >-
  READ-4: data.rs carries two comments that describe the opposite of what the
  code does — a phantom # Panics section and an inverted allocation claim
status: To Do
assignee:
  - TASK-1985
created_date: '2026-08-27 15:33'
updated_date: '2026-08-28 14:09'
labels:
  - code-review-rust
  - readability
dependencies: []
modified_files:
  - crates/extension/src/data.rs
  - crates/extension/src/extension.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/extension/src/data.rs:480-484` and `crates/extension/src/data.rs:506-509`; `crates/extension/src/data.rs:228-234`

**What**: two load-bearing comments in `data.rs` state the opposite of what the code below them does.

1. **A `# Panics` section for a panic that cannot happen.** `Context::get_or_provide` documents:

   > `# Panics` — If the `in_flight` entry inserted at the top of the call is missing by the time the provider returns — an internal invariant violation.

   The implementation is deliberately panic-free and says so three lines later:

   ```rust
   // The entry was inserted above and nothing between the two points
   // removes it, so `take` always hits; re-allocating the key is a
   // cost-free fallback that keeps this path panic-free.
   let owned_key = self.in_flight.take(key).unwrap_or_else(|| key.to_string());
   ```

   The two comments contradict each other in the same function. The `# Panics` heading is the one callers read, and it is wrong: this crate is compiled under a workspace policy that denies `panic`, `unwrap_used` and `panic_in_result_fn`, so a documented panic in a `pub fn` returning `Result` is a meaningful claim, and callers may wrap the call defensively on the strength of it. `clippy::missing_panics_doc` catches a *missing* section; nothing catches a surplus one.

2. **An allocation claim that is backwards.** `DataRegistry::register` documents its `Entry` migration as:

   > The audit-trail clone reuses the key already stored in the map on the duplicate path so the incoming allocation moves straight into the audit Vec without an extra copy.

   The code does the reverse — the incoming `name` is consumed by `entry(name)` and dropped, and the audit entry is a *clone* of the key already in the map:

   ```rust
   match self.providers.entry(name) {
       Entry::Occupied(occupied) => {
           ...
           self.duplicate_inserts.push(occupied.key().clone());
       }
   ```

   The duplicate path therefore performs exactly the heap copy the comment says it avoids. (The behaviour is fine and the `Entry` migration is still worth having for the single-probe reason; only the stated justification is false.)

**Why it matters**: READ-4 asks comments to explain *why*. A comment that explains a *why* that is not true is worse than none — the next reader either trusts it (and reasons from a false premise about panics or allocations) or discovers it is wrong and stops trusting the surrounding commentary, which in this file is dense and mostly excellent. Both are cheap to fix and both are the kind of drift that a rewrite leaves behind.

**Suggested fix**: delete the `# Panics` section from `get_or_provide` (or, if the invariant is worth asserting, keep the section and make the code match by returning an error variant — do not reintroduce a panic under the workspace's deny policy). Rewrite the `PATTERN-3 / TASK-1489` comment to state what the change actually bought: one hash probe instead of two on the happy path, at the cost of a key clone on the duplicate path. While there, check the parallel comment on `CommandRegistry::insert` (`crates/extension/src/extension.rs:128-134`), which makes a similar claim about the input `id`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Context::get_or_provide's rustdoc no longer documents a panic the implementation cannot produce
- [ ] #2 The PATTERN-3 comment on DataRegistry::register describes the actual cost profile: one probe on the happy path, a key clone on the duplicate path
- [ ] #3 The equivalent comment on CommandRegistry::insert is checked against its code and corrected if it makes the same inverted claim
<!-- AC:END -->
