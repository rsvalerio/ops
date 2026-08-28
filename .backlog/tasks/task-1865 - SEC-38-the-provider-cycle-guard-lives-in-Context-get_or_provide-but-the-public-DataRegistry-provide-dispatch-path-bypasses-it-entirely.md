---
id: TASK-1865
title: >-
  SEC-38: the provider-cycle guard lives in Context::get_or_provide, but the
  public DataRegistry::provide dispatch path bypasses it entirely
status: To Do
assignee:
  - TASK-1985
created_date: '2026-08-27 15:30'
updated_date: '2026-08-28 14:09'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - crates/extension/src/data.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/extension/src/data.rs:309-318` (also `crates/extension/src/data.rs:489-513`, `crates/extension/src/data.rs:267-269`)

**What**: SEC-38 / TASK-0744 added re-entrancy detection for circular data-provider dependencies, but it is implemented in `Context::get_or_provide` (the *caching* wrapper) rather than at the dispatch point:

```rust
// Context::get_or_provide — guard lives here
let owned_key = key.to_string();
if !self.in_flight.insert(owned_key) {
    return Err(DataProviderError::Cycle { key: key.to_string() });
}
let result = registry.provide(key, self);
```

```rust
// DataRegistry::provide — pub, no in_flight interaction at all
pub fn provide(&self, name: &str, ctx: &mut Context)
    -> Result<serde_json::Value, DataProviderError> {
    self.providers.get(name)
        .ok_or_else(|| DataProviderError::not_found(name))?
        .provide(ctx)
}
```

`DataRegistry::provide` is `pub`, takes the very `&mut Context` that owns the `in_flight` set, and never touches it. `DataRegistry::get` is also `pub` and hands out a `&dyn DataProvider` on which `provide(ctx)` can be called directly. So a provider composing other providers via `registry.provide(other, ctx)` — instead of `ctx.get_or_provide(other, registry)` — re-enters the provider graph with **no** re-entrancy marker and recurses to stack overflow on an A -> B -> A cycle, which is exactly the failure TASK-0744 was filed to close.

Nothing in the type system steers a caller to the guarded path. The only thing separating the safe entry point from the unsafe one is a prose note in `DataProvider::provide`'s rustdoc ("Implementations that compose other providers via `ctx.get_or_provide(...)` ..."). Per the skill's design philosophy: *if documentation is required to prevent misuse, the API is fragile.* This crate is the framework every extension crate builds against, so the fragile path is exposed to every current and future extension author.

**Why it matters**: a stack overflow is an abort, not a catchable error — the process dies with `SIGSEGV`/`thread ... has overflowed its stack` and no diagnostic naming the offending providers. The scenario is precisely the "misconfigured or hostile extension" threat `DataProviderError::Cycle` documents itself as defending against, so today the defence only holds for callers who happened to pick the right one of two equally public entry points.

**Suggested fix**: move the `in_flight` insert/remove into `DataRegistry::provide` itself (it already receives `&mut Context`, so it can own the marker) and reduce `Context::get_or_provide` to the cache fast-path plus a call into it. That makes the guard unconditional at the single dispatch point regardless of which public entry a caller reaches for. If keeping the guard in `Context` is preferred instead, demote `DataRegistry::provide` to `pub(crate)` and make `Context::get_or_provide` the only public way to invoke a provider (`DataRegistry::get` should then not expose a `provide`-callable trait object, or the trait method should be sealed).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 DataRegistry::provide (or whichever path remains public) marks the key in-flight before dispatching and clears it on both success and failure, so cycle detection cannot be bypassed by choosing a different public entry point
- [ ] #2 A regression test drives an A -> B -> A cycle through DataRegistry::provide directly (not through Context::get_or_provide) and asserts DataProviderError::Cycle instead of overflowing the stack
- [ ] #3 A regression test drives the same cycle through a &dyn DataProvider obtained from DataRegistry::get, or that path is closed off
- [ ] #4 Context::get_or_provide keeps its existing behaviour and all current tests in crates/extension/src/tests.rs still pass
<!-- AC:END -->
