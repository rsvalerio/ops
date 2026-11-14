---
id: TASK-2168
title: 'PERF-3: fix_trailing allocates and fills a full-size output buffer for every file, including the overwhelming majority that need no change'
status: Done
assignee: []
created_date: '2026-09-08 07:06'
updated_date: '2026-09-09 18:34'
labels:
  - code-review-rust
  - performance
dependencies: []
parent_task_id: 'TASK-2244'
modified_files:
  - extensions/text-fixers/src/trailing.rs
priority: low
ordinal: 81000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/text-fixers/src/trailing.rs:31`

**What**: `fix_trailing` opens with

```rust
let mut out = Vec::with_capacity(input.len());
```

and then copies every line into `out` while scanning, only to `return None`
and drop the whole buffer when `changed` is false. On a clean repository that
is one heap allocation plus one full `memcpy` of the file for *every* text
file discovered, all of it discarded.

`fix_eof` (`eof.rs`) does not have this shape — it scans first and only builds
a buffer once it knows the file changes.

**Why it matters**: the fixers run over every text file in the repository on
the `ops verify` and pre-commit paths, and the steady state after the first
run is that *nothing* changes — so the wasted copy is the common case, not the
edge case. `options.rs:8-18` also justifies the 16 MiB cap by peak memory:
"Both fixers hold the whole file in memory and `fix_trailing` allocates a
second buffer of the same size, so peak resident memory is roughly twice the
largest candidate". Deferring the allocation until a change is known removes
that doubling for clean files, which is every file on a passing run.

A scan-then-build split (or a first pass that finds the first line needing a
trim and only then allocates and copies the prefix) keeps the newline-count
invariant the module tests as a property, since the trimming logic is
unchanged.

<!-- scan confidence: verified — fix_trailing read in full -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 fix_trailing performs no heap allocation and no copy for an input that needs no change
- [x] #2 The newline-count invariant test (newline_count_is_invariant) and all existing trailing tests still pass unchanged
- [x] #3 The options.rs doc comment about peak memory is updated if the 2x claim no longer holds for clean files

<!-- AC:END -->
