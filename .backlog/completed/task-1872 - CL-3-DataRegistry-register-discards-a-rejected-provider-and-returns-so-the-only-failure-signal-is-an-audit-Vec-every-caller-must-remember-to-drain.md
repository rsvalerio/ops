---
id: TASK-1872
title: >-
  CL-3: DataRegistry::register discards a rejected provider and returns (), so
  the only failure signal is an audit Vec every caller must remember to drain
status: Done
assignee:
  - TASK-1985
created_date: '2026-08-27 15:31'
updated_date: '2026-08-28 19:24'
labels:
  - code-review-rust
  - api-design
dependencies: []
modified_files:
  - crates/extension/src/data.rs
  - crates/extension/src/extension.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/extension/src/data.rs:226-265` (`DataRegistry::register`, `take_duplicate_inserts`); sibling: `crates/extension/src/extension.rs:127-153` (`CommandRegistry::insert`)

**What**: `DataRegistry::register` is first-write-wins and returns `()`. When it rejects a duplicate it drops the incoming `Box<dyn DataProvider>` and pushes the name onto a private `duplicate_inserts` Vec:

```rust
pub fn register(&mut self, name: impl Into<String>, provider: Box<dyn DataProvider>) {
    ...
    Entry::Occupied(occupied) => {
        tracing::debug!(...);
        self.duplicate_inserts.push(occupied.key().clone());
    }
    ...
}
```

From the callsite, a rejected registration is indistinguishable from a successful one. The *only* way to learn that a provider was dropped is for some later, unrelated caller to remember to call `take_duplicate_inserts()` and do something with the result. That precondition — "after every batch of `register` calls, drain the audit trail" — exists nowhere in the types; it lives in prose spread across three rustdoc blocks and in the CLI wiring layer's discipline.

This is a precondition the compiler cannot check, and it has already failed once in production: DUP-3 / TASK-1225 had to retrofit a drain into `CommandRegistry`'s `FromIterator` impl because `collect()` consumers silently lost the same signal that ERR-2 / TASK-0579 had hardened `insert()` to preserve. `DataRegistry` has no `FromIterator` impl today, so the identical hole reopens the moment anyone adds one, or adds any other batch-registration helper.

The asymmetry with the sibling registry compounds it (READ-6, consistent patterns for similar problems): `CommandRegistry::insert` returns `Option<CommandSpec>` — the outcome is visible in the return value *and* recorded in the audit trail — while `DataRegistry::register` returns nothing at all, even though its rejection is the more consequential of the two (a dropped provider means a data source silently missing at query time, versus a shadowed command which at least still runs *something*).

**Why it matters**: the audit trail is a strictly weaker mechanism than a return value: it is drained by a different piece of code than the one that made the mistake, at a time the caller does not control, and forgetting to drain it is silent. A dropped provider surfaces much later as a `DataProviderError::NotFound` from an unrelated command with no hint that a registration was refused. Encoding the outcome in the return type makes the wrong thing impossible to write without acknowledging it.

**Suggested fix**: return the rejected value — `pub fn register(...) -> Option<Box<dyn DataProvider>>` (mirroring `CommandRegistry::insert`'s shape) or `Result<(), DuplicateProvider>`. Mark it `#[must_use = "a returned provider was rejected as a duplicate and dropped"]` so ignoring the outcome is a compile-time warning rather than an invisible default. Keep `take_duplicate_inserts` for the aggregated warning the wiring layer emits; it becomes a convenience rather than the sole channel. The two registries should then document the shared shape in one place instead of two divergent policy essays.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 DataRegistry::register communicates rejection through its return type, annotated #[must_use] with a message naming the action the caller forgot
- [ ] #2 Existing callers across the workspace are updated to acknowledge the return value; the aggregated take_duplicate_inserts warning path in the CLI wiring layer still works
- [ ] #3 A test asserts the return value identifies the rejected provider on a duplicate and is the no-op variant on a fresh insert
- [ ] #4 The first-write-wins vs last-write-wins policy split between DataRegistry and CommandRegistry is documented once, in one place, rather than duplicated in both rustdoc blocks
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
DataRegistry::register now returns Option<Box<dyn DataProvider>> (Some = the rejected incoming provider) and is #[must_use] with a message naming the action (AC#1). All 39 call sites across the workspace acknowledge it with `let _ =`; the CLI wiring path in registration.rs carries a comment explaining why the discard is correct there, and take_duplicate_inserts still feeds the aggregated warning (AC#2). Test register_returns_none_on_fresh_insert_and_the_rejected_provider_on_duplicate (AC#3). The first-write-wins vs last-write-wins split is now documented once in the new ops_extension::registry_duplicate_policy doc module; both methods link to it instead of restating it (AC#4).
<!-- SECTION:NOTES:END -->
