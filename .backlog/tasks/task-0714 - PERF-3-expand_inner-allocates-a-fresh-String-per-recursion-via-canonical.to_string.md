---
id: TASK-0714
title: >-
  PERF-3: expand_inner allocates a fresh String per recursion via
  canonical.to_string()
status: To Do
assignee:
  - TASK-0741
created_date: '2026-04-30 05:30'
updated_date: '2026-04-30 06:07'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/resolve.rs:140-147`

**What**: `expand_inner` tracks the active recursion stack with `visited: &mut HashSet<String>` and on every composite step performs `visited.insert(canonical.to_string())` followed by `visited.remove(canonical)` after the children are expanded. `canonical` is already a `&str` borrowed from the IndexMap key (stable for the duration of the borrow), but the HashSet stores owned `String`s, so each composite visited allocates a new `String` even though the canonical name is already interned in `self.config.commands` / `self.stack_commands` / `self.extension_commands`. For a plan with C composite levels in a workspace with many sub-composites this is C heap allocations per `expand_to_leaves` call, exactly the pattern OWN-8 calls out (cloning to satisfy the borrow checker).

**Why it matters**: `expand_to_leaves` runs at every `ops <cmd>` invocation before any work happens. The allocations are individually tiny but they tax the hot path on every CLI invocation, and the design also masks a real ownership question: the visited set tracks names that already live in the config — a borrowed-key set (`HashSet<&str>` with the lifetime of the runner) or storing CommandId (which is a small refcounted Arc per TASK-0658-style refactors) would eliminate the allocation while making the lifetime explicit.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace HashSet<String> with HashSet<&str> bound to the runner-borrow lifetime (or with a CommandId-keyed set if the type already exposes Arc-cheap clones) so canonical.to_string() is no longer required per recursion
- [ ] #2 Bench/microbench (or trace) confirms zero String allocations in expand_inner for a 10-deep composite plan
<!-- AC:END -->
