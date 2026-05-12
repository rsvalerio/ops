---
id: TASK-1323
title: >-
  READ-7: run_hook_dispatch parameter named run_preflight but receives the
  --changed-only CLI flag, conflating two distinct concepts
status: Done
assignee:
  - TASK-1382
created_date: '2026-05-11 20:56'
updated_date: '2026-05-12 22:59'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/subcommands.rs:159-211`

**What**: `run_hook_dispatch` declares its third parameter as `run_preflight: bool` and uses it to gate the preflight predicate (`has_staged_files` for the commit hook). The single production call site `run_hook_action` (line 211) passes `changed_only` — the user-facing `--changed-only` CLI flag from `RunBeforeCommit { changed_only, .. }` / `RunBeforePush { changed_only, .. }` — into that slot:

```rust
fn run_hook_action(... changed_only: bool) -> ... {
    ...
    run_hook_dispatch(config, hook, changed_only)  // <-- changed_only -> run_preflight
}
```

So the parameter named "run_preflight" at the callee is the user flag "changed_only" at the caller. These are not the same concept:
- `--changed-only` is a *user-facing* execution-scope flag: "only check staged/changed files instead of the entire workspace" (per the doc comments at args.rs:106-108 and 118-120).
- `run_preflight` is an *internal* dispatch toggle: "do/don't invoke `has_staged_files` before spawning the command".

The implementation collapses them because the only preflight that exists today (`has_staged_files`) happens to be the same predicate a `--changed-only` consumer would short-circuit on. That coincidence is load-bearing: the moment a future hook adds an unrelated preflight (e.g. `network_available`, `ci_environment`) the wiring breaks silently, because that preflight will now run iff the user passed `--changed-only` — which has nothing to do with it.

**Why it matters**:
1. The naming actively misleads. Reading `run_hook_dispatch` in isolation, you'd believe its caller decided whether to run preflights; in reality the user did, via a flag whose docstring promises something else.
2. This is the root of the silent-no-op behaviour TASK-1307 and TASK-1308 (already on file) describe at the *symptom* level. Those tasks ask for `--changed-only` to actually constrain the command's file scope; this task asks for the underlying type to split the two concerns so the symptom can't reappear under a different name.
3. The variable rename mid-call (`changed_only` -> `run_preflight`) is exactly the swap-bug surface that FN-3/RunOptions/PlanShape were introduced to eliminate elsewhere in this crate (see TASK-0272, TASK-0866).

This is distinct from TASK-1307/1308 — those file the *user-visible behaviour bug*; this is the *internal API shape* that lets the bug recur.

Fix: rename the parameter to `changed_only` at the callee and decide preflight policy from `HookOps` + `changed_only` explicitly (e.g. `if changed_only && hook.preflight.is_some() { ... }`), so the relationship is spelled out in code rather than smuggled through identical bool slots.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Rename run_hook_dispatch's third parameter to match the caller's semantic (changed_only)
- [ ] #2 Make the preflight-vs-changed-only relationship explicit at the dispatch site instead of folding both onto one bool
- [ ] #3 Add a unit test that demonstrates a preflight unrelated to file-change scope (e.g. a stub returning false unconditionally) is not silently gated by changed_only
<!-- AC:END -->
