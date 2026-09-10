---
id: TASK-2258
title: 'DUP-1: ops_about::lru::lock_recovering is a second workspace statement of the ops_core::sync poison-recovery policy'
status: Triage
assignee: []
created_date: '2026-09-10 18:41'
labels:
  - code-review-rust
  - duplication
dependencies: []
modified_files:
  - extensions/about/src/lru.rs
  - crates/core/src/sync.rs
  - extensions-rust/about/src/manifest_cache.rs
  - extensions-rust/about/src/coverage_provider.rs
  - extensions-rust/about/src/workspace_root_cache.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/lru.rs:405` (`lock_recovering`), duplicating `crates/core/src/sync.rs:30` (`lock_recover`) / `:46` (`lock_recover_warn`)

**What**: TASK-2150 lifted the three ops-about-rust caches onto `BoundedLruCache` and, with them, a single `pub fn lock_recovering<T: ?Sized>(lock: &Mutex<T>, on_poison: impl FnOnce()) -> MutexGuard<'_, T>`. That helper is a fourth spelling of a policy `ops-core` already owns: `lock_recover` is the silent-recovery form (identical body: `lock().unwrap_or_else(|e| { clear_poison(); e.into_inner() })`), and `lock_recover_warn` is the breadcrumb form. `ops-about` already depends on `ops-core` (`extensions/about/Cargo.toml:17`), so the dependency edge needed for reuse exists today.

Two of `lock_recovering`'s three callers pass `|| {}` (`coverage_provider.rs:115`, `workspace_root_cache.rs:95`), i.e. exactly `lock_recover`; only `manifest_cache.rs:248` supplies a real warn closure.

**Why it matters**: the workspace now states its mutex-poisoning policy in two crates. `crates/core/src/sync.rs` documents the silent-vs-breadcrumb split as the deliberate policy, and a second implementation outside that module means a future change to the policy (e.g. deciding poison should always warn) upgrades only one half. This is the same class of drift TASK-2150 itself was filed to remove, reintroduced one level up.

The blocker to a drop-in reuse is real but small: `lock_recover_warn` is `#[cfg(test)]`-gated (`crates/core/src/sync.rs:45`) because all its callers were tests, and `lock_recover` is `<T>` rather than `<T: ?Sized>`. Resolving this likely means ungating the warn form (or giving it a hook parameter) and relaxing the bound, then deleting `lock_recovering`.

**Origin**: discovered during TASK-2242 (wave 8) while auditing TASK-2150; it is a side effect of that wave's own fix, and it spans two crates' public surface so it was not folded into the wave.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The poison-recovery policy is stated once in the workspace; ops_about::lru does not define its own lock helper
- [ ] #2 The three BoundedLruCache callers reach the ops_core helper directly, with the one warn-emitting caller keeping its breadcrumb
<!-- AC:END -->
