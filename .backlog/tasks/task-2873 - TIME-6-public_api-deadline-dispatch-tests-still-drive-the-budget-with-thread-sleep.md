---
id: TASK-2873
title: 'TIME-6: public_api deadline dispatch tests still drive the budget with thread::sleep'
status: Triage
assignee: []
created_date: '2026-11-18 22:11'
labels:
  - code-review-rust
  - TIME
dependencies: []
modified_files:
  - crates/extension/tests/public_api.rs
  - crates/extension/src/context.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/extension/tests/public_api.rs` (SlowProvider, an_over_budget_provider_keeps_its_own_error, nested_dispatch_inherits_the_outermost_deadline, the detached-handle test), `crates/extension/src/context.rs` (Context::begin_deadline)

**What**: Deadline now exposes `is_expired_at(now)` / `check_at(now)` and its boundaries are unit-tested without sleeping, but the dispatch-level tests in `tests/public_api.rs` still exercise the budget through `Context`, which reads `Instant::now()` in `begin_deadline` and in the no-arg `check`. They keep 40-120ms wall-clock sleeps against 20ms budgets.

**Why it matters**: TIME-6 / TEST-16: the sleeps add real latency to the suite and depend on scheduler timing; a clock seam on `Context` (e.g. under `test-support`) would let them run against a fixed instant.

**Origin**: discovered during TASK-2844 while fixing TASK-2698.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Context deadline installation and checks can be driven by an injected clock (test-support)
- [ ] #2 The public_api deadline dispatch tests run without std::thread::sleep
<!-- AC:END -->
