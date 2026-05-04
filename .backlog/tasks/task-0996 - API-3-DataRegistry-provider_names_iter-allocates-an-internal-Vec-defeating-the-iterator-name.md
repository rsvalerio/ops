---
id: TASK-0996
title: >-
  API-3: DataRegistry::provider_names_iter allocates an internal Vec, defeating
  the iterator name
status: Triage
assignee: []
created_date: '2026-05-04 22:00'
labels:
  - code-review-rust
  - structure
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/extension/src/data.rs:200-209`

**What**: `provider_names_iter()` is doc-commented as "zero-allocation
iteration over registered provider names in sorted order", but the body
collects every key into a heap `Vec<&str>`, sorts it, and returns
`names.into_iter()`. Every call allocates a `Vec` proportional to the
registry size. The sibling `provider_names()` method delegates to
`provider_names_iter().collect()` and pays the same cost twice (once for
the inner Vec, once for the outer collect).

**Why it matters**: The name and rustdoc promise a streaming iterator;
callers who choose `_iter` over the `Vec`-returning sibling expect to
avoid the allocation. The current shape gives them all of the cost
(sort + Vec) plus the indirection of an iterator wrapper. The audit-trail
seed in `crates/cli/src/registry/registration.rs:141` (`seed_owners(
registry.provider_names().into_iter().map(str::to_string))`) hits the same
double-allocation: it could use `provider_names_iter()` directly today,
but only because that method secretly allocates anyway.

Either rename (`provider_names_sorted` returning `Vec`) or actually
stream sorted output via a `BTreeSet`-backed iterator constructed from
`self.providers.keys()`.

A close PERF-3 cousin: in `registration.rs:141`, prefer
`provider_names_iter().map(str::to_string)` over
`provider_names().into_iter().map(...)` so a future fix to
`provider_names_iter` benefits the registration audit path automatically.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 provider_names_iter does not allocate an internal Vec, OR is renamed to reflect its allocation behaviour
- [ ] #2 callers in cli/src/registry/registration.rs use the non-allocating accessor
<!-- AC:END -->
