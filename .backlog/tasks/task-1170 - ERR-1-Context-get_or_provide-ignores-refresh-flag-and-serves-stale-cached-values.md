---
id: TASK-1170
title: >-
  ERR-1: Context::get_or_provide ignores refresh flag and serves stale cached
  values
status: To Do
assignee:
  - TASK-1268
created_date: '2026-05-08 08:06'
updated_date: '2026-05-08 13:19'
labels:
  - code-review-rust
  - err
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/extension/src/data.rs:377`

**What**: `get_or_provide` returns from `data_cache` without consulting `self.refresh`, even though the field is documented as "providers should re-collect data instead of using cached/persisted results". A `Context` constructed with `with_refresh()` still serves any value already cached on this context.

**Why it matters**: `cargo ops <cmd> --refresh` (or any caller setting `refresh=true`) silently reuses the prior run's results for any provider whose value was previously cached on the same `Context`. Together with TASK-0993 folding the cache onto the persistent runner Context, refresh is effectively a no-op for repeat queries within a runner lifetime.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 When ctx.refresh is true, get_or_provide bypasses the data_cache.get fast path and re-invokes registry.provide, then updates the cache.
- [ ] #2 Regression test constructs a Context, primes a key via get_or_provide, sets refresh = true (or builds via with_refresh), and asserts the provider is invoked a second time.
<!-- AC:END -->
