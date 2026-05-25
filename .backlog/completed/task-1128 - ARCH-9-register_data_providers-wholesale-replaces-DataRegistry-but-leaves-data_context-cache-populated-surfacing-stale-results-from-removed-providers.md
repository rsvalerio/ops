---
id: TASK-1128
title: >-
  ARCH-9: register_data_providers wholesale-replaces DataRegistry but leaves
  data_context cache populated, surfacing stale results from removed providers
status: Done
assignee:
  - TASK-1262
created_date: '2026-05-08 07:30'
updated_date: '2026-05-08 15:33'
labels:
  - code-review-rust
  - architecture
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: \`crates/runner/src/command/mod.rs:305-307\` (and ARCH-9 sibling TASK-0993)

**What**: \`pub fn register_data_providers(&mut self, registry: DataRegistry) { self.data_registry = registry; }\` swaps the registry but does not touch \`self.data_context\`, which carries the provider-result cache TASK-0993 consolidated. If a caller calls \`query_data("foo")\` (populating the cache), then re-registers with a registry that *no longer contains* \`"foo"\` (or contains a new provider implementation under the same name), subsequent \`query_data("foo")\` returns the stale cached \`Arc<serde_json::Value>\` from the previous registry — including for a provider that has been intentionally replaced or removed.

**Why it matters**: TASK-0993 made \`data_context\` the single source of truth for the cache; that consolidation only holds while the registry it was populated against is also the live registry. Today the only production caller (CLI dispatch) registers once at startup, but the API is public (\`pub fn register_data_providers\`) and tests / embedders that re-register would silently observe the previous registry's results. Symmetric with TASK-0904's "first-write-wins vs last-write-wins" classification: registry replace + cache survival is "first-call-wins" forever for any cached key.

**Fix**: clear the \`data_context\` cache on \`register_data_providers\` (or rebuild from \`Context::from_cwd_arc\`), document the contract that re-registration invalidates cached results, and add a regression test exercising the swap.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 register_data_providers either clears data_context's cache or rebuilds the Context so post-swap query_data sees the new registry's providers
- [ ] #2 Doc comment on register_data_providers states the cache invalidation contract
- [ ] #3 Regression test: register P1 -> query_data("x") -> register P2 (different impl of "x") -> query_data("x") returns P2's value, not P1's cached one
<!-- AC:END -->
