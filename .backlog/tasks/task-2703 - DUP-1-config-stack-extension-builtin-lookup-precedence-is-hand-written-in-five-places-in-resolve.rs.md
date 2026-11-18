---
id: TASK-2703
title: 'DUP-1: config → stack → extension → builtin lookup precedence is hand-written in five places in resolve.rs'
status: To Do
assignee: []
created_date: '2026-11-18 18:31'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - DUP
dependencies: []
parent_task_id: 'TASK-2842'
modified_files:
  - crates/runner/src/command/resolve.rs
priority: low
ordinal: 1000
dedup_key: 'DUP-1:crates/runner/src/command/resolve.rs:canonical_with_spec'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/resolve.rs:235` (`find_in_stores`), `resolve.rs:262` (`canonical_with_spec`, three copies of the chain: direct id, config alias target, non-config alias target), `resolve.rs:318` (`locate`), `resolve.rs:333` (`resolve_alias`, two more copies)

**What**: The store precedence (config, then stack, then extension, then builtin) is spelled out as a separate `get` / `get_key_value` chain in each function. `canonical_with_spec` alone repeats the four-step `if let Some((k, v)) = store.get_key_value(..)` block three times, and `resolve` / `resolve_alias` re-implement what `canonical_with_spec(id).map(|(_, s)| s)` already returns.

**Why it matters**: The order is a behaviour contract (builtins land last so config, stack and extensions can shadow them; `locate` reports the source). Adding a store, or changing precedence, means editing every copy the same way. A copy missed in one place would make `resolve`, `locate` and composite expansion disagree on which spec an id means.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The store precedence is defined once (e.g. a helper iterating the stores in order, returning key, spec and CommandSource) and find_in_stores / canonical_with_spec / resolve_alias / locate are built on it
- [ ] #2 Existing resolution, alias and shadowing tests pass unchanged
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.05 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
