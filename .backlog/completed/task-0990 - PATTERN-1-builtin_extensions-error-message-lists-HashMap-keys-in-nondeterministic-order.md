---
id: TASK-0990
title: >-
  PATTERN-1: builtin_extensions error message lists HashMap keys in
  nondeterministic order
status: Done
assignee: []
created_date: '2026-05-04 21:59'
updated_date: '2026-05-04 23:17'
labels:
  - code-review-rust
  - structure
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/registry/discovery.rs:106-113`

**What**: When an `extensions.enabled` entry is not compiled in, the error
joins `available.keys().cloned().collect::<Vec<_>>().join(", ")` from a
`HashMap<&'static str, Box<dyn Extension>>`. `HashMap` iteration order is
randomised per process (and across versions), so the list of "available"
extensions appears in a different order on each invocation and across
machines.

**Why it matters**: User-visible error messages with shuffled token order are
hostile to (a) snapshot/integration tests, (b) operators copy-pasting the
list into bug reports, and (c) anyone trying to grep for "did extension X
appear?" because the substring position drifts. The same pattern was already
fixed elsewhere (`DataRegistry::provider_names_iter` sorts by name).

Sort the keys before joining (e.g. collect into a `Vec`, `sort_unstable`,
then `join`) so the message is deterministic and skim-able.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 available extension list in the bail! message is sorted alphabetically
- [ ] #2 regression test pins the sorted order across two consecutive invocations
<!-- AC:END -->
