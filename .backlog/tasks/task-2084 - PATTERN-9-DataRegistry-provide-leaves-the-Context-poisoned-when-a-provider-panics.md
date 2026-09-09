---
id: TASK-2084
title: 'PATTERN-9: DataRegistry::provide leaves the Context poisoned when a provider panics'
status: Done
assignee: []
created_date: '2026-09-07 22:58'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-rust
  - idioms
dependencies: []
parent_task_id: 'TASK-2243'
modified_files:
  - crates/extension/src/data.rs
priority: medium
ordinal: 12000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/extension/src/data.rs:539-543`

**What**: `DataRegistry::provide` sets up state around the dispatch — `ctx.enter_provider(name)` inserts the in-flight marker, `ctx.begin_deadline(name)` may install the dispatch deadline — and tears it down with `ctx.clear_deadline_if_owned(owns_deadline)` / `ctx.exit_provider(name)` *after* `provider.provide(ctx)` returns. There is no drop guard or unwind handler: if a provider **panics** (providers are extension-supplied code; out-of-tree implementations cannot be audited by this workspace's `panic = "deny"` lint), neither teardown call runs.

**Why it matters**: `Context` is long-lived — `get_or_provide`'s docs (TASK-0993) record that the cache was folded onto the *persistent runner Context*, which lives across repeat queries. One panicking provider permanently poisons it:

- the leaked `in_flight` marker makes every later request for that key return `DataProviderError::Cycle { key }` — a cycle that does not exist, misdirecting whoever reads the log;
- if the panicking provider owned the deadline, the stale `Deadline` survives, and `begin_deadline` returns `false` for every later dispatch (`self.deadline.is_some()`), so all subsequent dispatches on this Context inherit a deadline owned by the dead provider and can be rejected as `TimedOut` naming it.

The docs on `exit_provider` claim the marker is cleared "on both the success and the failure path" — panic is a third path. PATTERN-9: cleanup repeated at each exit belongs in a value whose `Drop` runs it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A provider panic during DataRegistry::provide no longer poisons the context: a later dispatch of the same key succeeds (or fails for its own reason), not Cycle
- [ ] #2 No stale deadline remains after a panicked dispatch: ctx.deadline() is None between dispatches and a later dispatch installs its own
- [ ] #3 Teardown (exit_provider + clear_deadline_if_owned) is performed by a Drop guard or equivalent unwind-safe mechanism, not only fall-through code
- [ ] #4 Regression test: a panicking provider, unwound via catch_unwind around registry.provide, followed by a second provide of the same key that succeeds
<!-- AC:END -->
