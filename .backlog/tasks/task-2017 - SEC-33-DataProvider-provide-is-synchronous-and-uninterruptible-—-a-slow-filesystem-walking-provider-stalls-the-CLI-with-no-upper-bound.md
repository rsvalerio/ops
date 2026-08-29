---
id: TASK-2017
title: >-
  SEC-33: DataProvider::provide is synchronous and uninterruptible — a slow
  filesystem-walking provider stalls the CLI with no upper bound
status: To Do
assignee:
  - TASK-2047
created_date: '2026-08-28 15:58'
updated_date: '2026-08-29 11:35'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - crates/extension/src/data.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/extension/src/data.rs` (the `DataProvider` trait), reached from `extensions/tokei/src/lib.rs` and `extensions-rust/loc/src/lib.rs`

**What**: `DataProvider::provide` is a synchronous method that runs inline on
the calling thread with no deadline and no cancellation point. Providers that
walk the operator's working directory (`tokei`, `rust-loc`, `text-fixers`) can
therefore occupy the process for as long as the tree takes, and there is no
mechanism at the trait level for a caller to bound or interrupt one.

TASK-1970 capped the *work* the tokei provider will do -- per-file byte cap,
file-count cap, walk depth -- which removes the unbounded-memory half of the
problem and makes the walk finite. It does not give the caller a time bound:
50k files on a slow or network filesystem is still an arbitrary wall-clock
stall, and the same is true of every other walking provider.

**Why it matters**: SEC-33 -- resource exhaustion. These statistics are
advisory display data; none of them is worth an unbounded stall on input the
operator did not author. A bound has to live where the dispatch happens, not in
each provider, or every new provider re-acquires the defect.

**Origin**: discovered during TASK-2012 while fixing TASK-1970 (the fourth
bullet of that finding, which is out of scope for a single extension crate).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Provider dispatch enforces a bound on how long a single provide() call may run, or the trait exposes a cancellation signal providers are required to honour
- [ ] #2 Exceeding the bound is reported as a provider failure with the provider name, not as an empty or partial success
- [ ] #3 A test drives a deliberately slow provider through the dispatch path and asserts the bound is enforced
<!-- AC:END -->
