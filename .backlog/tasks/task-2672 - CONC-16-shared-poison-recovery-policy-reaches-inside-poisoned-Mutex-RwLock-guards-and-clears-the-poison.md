---
id: TASK-2672
title: 'CONC-16: shared poison-recovery policy reaches inside poisoned Mutex/RwLock guards and clears the poison'
status: To Do
assignee: []
created_date: '2026-11-18 18:26'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - CONC
dependencies: []
parent_task_id: 'TASK-2839'
modified_files:
  - crates/core/src/sync.rs
  - crates/core/src/config/loader/global.rs
  - crates/core/src/expand.rs
  - crates/core/src/stack/detect.rs
priority: medium
ordinal: 1000
dedup_key: 'CONC-16:crates/core/src/sync.rs:lock_recover_with'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/sync.rs:43-52` (`lock_recover_with`, and its wrappers `lock_recover` / `lock_recover_warn`), `crates/core/src/config/loader/global.rs:58-79` (`write_global_config_path`, `global_config_path`)

**What**: `sync::lock_recover_with` matches `Err(poison)`, calls `m.clear_poison()` and returns `poison.into_inner()`; `lock_recover` does so silently. It is the crate's (and, per its module docs, the workspace's) single statement of the policy, used by production caches in `expand.rs` (`cached_ops_root_arc`, `mark_expand_warn_emitted`) and `stack/detect.rs` (`canonicalize_cached`). `config/loader/global.rs` repeats the pattern for an `RwLock`: `GLOBAL_CONFIG_PATH.write().unwrap_or_else(|p| { clear_poison(); p.into_inner() })` and `read().unwrap_or_else(PoisonError::into_inner)`.

The justification in the docs ("every state of the protected map is valid") is the exemption CONC-16 explicitly rejects: it is a claim about today's critical sections (e.g. `cached_ops_root_arc` already does a multi-step map+`order` VecDeque update whose two halves can desynchronise if a panic lands between them) that no one re-checks when a section grows.

<!-- scan confidence: policy-level finding; the recovery is documented and deliberate, so triage should decide whether to adopt CONC-16 or record an explicit exemption -->

**Why it matters**: A poisoned lock means a holder panicked mid-update; recovering silently hands later callers possibly half-updated state (e.g. a `map`/`order` pair out of sync in the FIFO caches) and erases the evidence. Propagating with `.expect("… poisoned")` lets the panic surface through the normal failure path instead.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either poisoned locks are propagated (.lock().expect("<name> poisoned")) at every listed site, or a recorded decision documents the exemption from CONC-16 with its scope
- [ ] #2 No production site calls clear_poison() or PoisonError::into_inner without that decision
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.03 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
