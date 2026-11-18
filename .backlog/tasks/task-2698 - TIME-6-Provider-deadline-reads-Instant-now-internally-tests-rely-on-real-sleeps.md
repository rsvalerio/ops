---
id: TASK-2698
title: 'TIME-6: Provider deadline reads Instant::now() internally; tests rely on real sleeps'
status: To Do
assignee: []
created_date: '2026-11-18 18:29'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - TIME
dependencies: []
parent_task_id: 'TASK-2844'
modified_files:
  - crates/extension/src/deadline.rs
  - crates/extension/src/context.rs
  - crates/extension/tests/public_api.rs
priority: low
ordinal: 1000
dedup_key: 'TIME-6:crates/extension/src/deadline.rs:Deadline::is_expired'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
<!-- scan confidence: candidates to inspect -->
**File**: `crates/extension/src/deadline.rs:83-85` (`Deadline::is_expired`), `crates/extension/src/context.rs:331` (`Context::begin_deadline`), `crates/extension/src/data.rs:403` (overrun check in `DataRegistry::provide`)

**What**: The dispatch budget makes its expiry *decision* by reading `Instant::now()` directly inside `Deadline::is_expired` and `Context::begin_deadline`, with no clock injection point. As a result every deadline test in `crates/extension/tests/public_api.rs` drives the bound with wall-clock `std::thread::sleep` against 20ms budgets: `SlowProvider::provide` (~line 1530, 20 x 10ms), `an_over_budget_provider_keeps_its_own_error` (~1606, 60ms), `nested_dispatch_inherits_the_outermost_deadline` (~1703, 120ms), and the detached-handle test (~1919, 40ms).

**Why it matters**: TIME-6: a clock read in decision logic cannot be tested at the boundary (exactly at expiry, +/-1 tick) and forces sleep-based tests that add real latency to the suite and depend on scheduler timing. The current sleeps all overshoot the budget, so they are slow rather than flaky today, but the "expires exactly at the budget" and "does not expire just before it" boundaries are untested.

**Suggested fix**: let `Deadline` take `now: Instant` (e.g. `is_expired_at(now)` / `check_at(now)`) with the public no-arg methods reading the clock once at the edge, or inject a small clock into `Context` under `test-support`, and rewrite the deadline tests against a fixed instant.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Deadline expiry logic can be evaluated against a caller-supplied Instant (or an injected clock)
- [ ] #2 Deadline tests cover the at-expiry and just-before-expiry boundaries without thread::sleep
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.04 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
