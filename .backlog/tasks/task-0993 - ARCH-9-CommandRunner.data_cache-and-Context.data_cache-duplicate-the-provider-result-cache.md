---
id: TASK-0993
title: >-
  ARCH-9: CommandRunner.data_cache and Context.data_cache duplicate the
  provider-result cache
status: Triage
assignee: []
created_date: '2026-05-04 21:59'
labels:
  - code-review-rust
  - structure
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/mod.rs:120-121, 254-269`

**What**: `CommandRunner` holds its own `data_cache: HashMap<String,
Arc<serde_json::Value>>` and, on every `query_data` call, constructs a
fresh `ops_extension::Context` (which has its **own** `data_cache` field)
via `Context::from_cwd_arc`. `ctx.get_or_provide` populates the per-call
context cache, then `query_data` copies the result into
`self.data_cache`. The context's cache is dropped immediately because the
context is throw-away.

That means:

1. Every entry that involves transitive `ctx.get_or_provide(...)` calls
   populates a fresh per-query context cache that nothing reuses; only the
   *outermost* key flows back to `self.data_cache`. Composed providers pay
   recompute cost on every outer query.
2. The two caches can drift if a provider ever inserts via `Context` but
   the runner did not request that key directly (cycle-detection state in
   `Context.in_flight` is also throw-away, so two parallel `query_data`
   calls for the same key cannot share cycle detection — one would not see
   the other's in-flight marker).

**Why it matters**: This is the kind of dual-state cache that survives
exactly until someone composes providers. The CLI today only requests one
top-level key per `query_data`, so the bug is dormant; the moment an
extension publishes a provider that internally calls `ctx.get_or_provide`
to fan out to peers (the documented composition pattern in
`crates/extension/src/data.rs:104-108`), the inner results vanish on every
outer query.

Either: (a) keep one cache by passing `&mut self.data_cache` into a
context that holds `&mut HashMap<...>` instead of owning its own, or (b)
fold `query_data` into a helper that promotes the populated context cache
back into `self.data_cache` after each call.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 single source of truth for the per-runner data cache (no duplicated HashMap)
- [ ] #2 regression test: a provider that calls ctx.get_or_provide(other) only computes 'other' once across two outer query_data calls in the same runner
<!-- AC:END -->
