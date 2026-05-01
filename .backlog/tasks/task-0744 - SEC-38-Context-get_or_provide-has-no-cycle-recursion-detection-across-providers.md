---
id: TASK-0744
title: >-
  SEC-38: Context::get_or_provide has no cycle/recursion detection across
  providers
status: Triage
assignee: []
created_date: '2026-05-01 05:52'
labels:
  - code-review-rust
  - security
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/extension/src/data.rs:292-304`

**What**: `get_or_provide` checks the cache then calls `registry.provide(key, self)`. A provider that internally calls `ctx.get_or_provide(other_key, registry)` (the documented composition pattern) can transitively re-enter `get_or_provide` for the original `key` before the cache entry is inserted. There is no `visiting`/depth guard analogous to `Config::walk_composite`.

**Why it matters**: A misconfigured or hostile extension that registers circular provider dependencies (A → B → A) recurses until stack overflow, taking down the CLI process. Cycles are not detected at registration time and not at request time.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 get_or_provide rejects re-entrant requests for an in-flight key with a typed error (e.g. DataProviderError::Cycle { key })
- [ ] #2 Regression test wires up two providers that mutually request each other and asserts the cycle surfaces as an error rather than overflowing the stack
- [ ] #3 Document the new error variant on DataProvider::provide and at get_or_provide call sites
<!-- AC:END -->
