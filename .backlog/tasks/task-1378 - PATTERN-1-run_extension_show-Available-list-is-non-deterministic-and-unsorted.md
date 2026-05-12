---
id: TASK-1378
title: >-
  PATTERN-1: run_extension_show 'Available:' list is non-deterministic and
  unsorted
status: Done
assignee:
  - TASK-1383
created_date: '2026-05-12 21:52'
updated_date: '2026-05-12 23:16'
labels:
  - code-review-rust
  - architecture
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/extension_cmd.rs:278-288`

**What**: When the user runs `ops extension show <name>` for a name not present in the compiled set, the error message renders the available extensions via `compiled.iter().map(|(n, _)| *n)` and `available.join(", ")`. `compiled` is the `Vec` returned by `collect_compiled_extensions`, whose order is the `EXTENSION_REGISTRY` distributed-slice slot order — i.e. link-time-dependent and not sorted. The sister error path in `discovery.rs::builtin_extensions` (TASK-0990, marked Done) already collects and `.sort_unstable()` for exactly this reason: snapshot-friendly, copy-pasteable, deterministic across builds.

Sample current output for a missing name:
```
extension not found: foo. Available: cargo-toml, git, metadata, tools, ...
```
The list order depends on slot order in `ops_extension::EXTENSION_REGISTRY`, which is determined by `extern crate ops_*` declaration order in `main.rs`. Operators reading the error in a bug report can't reproduce the list deterministically.

**Why it matters**: Mirrors the TASK-0990 fix on the sister error path; the show-error inherited the bug. Low impact on the happy path, but breaks the deterministic-error-message contract operators rely on.

**Fix**: collect into `Vec<&str>`, `sort_unstable()`, then `join(", ")`. One-line change mirroring discovery.rs:124-128.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 run_extension_show error path sorts available names before joining
- [ ] #2 Two consecutive failed show-calls produce byte-identical error messages
- [ ] #3 Unit test added to extension_cmd::tests pinning the sort order
<!-- AC:END -->
