---
id: TASK-1583
title: >-
  PATTERN-1: RUSTUP_TARGET_ARCH_PATTERNS hardcoded triple list drifts as rustup
  adds targets
status: Done
assignee:
  - TASK-1637
created_date: '2026-05-21 22:45'
updated_date: '2026-05-22 12:51'
labels:
  - code-review-rust
  - patterns
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/tools/src/probe/rustup.rs:89-130`

**What**: `strip_target_triple` splits component names from target triples by linear-scanning a hardcoded `const RUSTUP_TARGET_ARCH_PATTERNS` array of `-aarch64-`, `-x86_64-`, etc. Any new architecture rustup grows support for (e.g. future RISC-V variants, ARM big.LITTLE additions, embedded targets) is invisible to `is_component_in_list`, so an installed component on the new triple will be reported as not installed and trigger a redundant `rustup component add`.

**Why it matters**: PATTERN-1 — derive the parse from rustup's contract (the component head ends at the first known target triple, which rustup itself enumerates via `rustup target list`) rather than mirroring an enumerable closed set in code. The linear scan is also O(n) per line where n is the pattern count; for component-list output of K lines this is O(K·n) and grows with every rustup release. At minimum, add a CI test that asserts no installed target triple is missing from the pattern list.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 strip_target_triple no longer depends on a hand-curated arch list, OR the list is validated against the active rustup's known triples in a CI test
- [x] #2 is_component_in_list tests cover at least one component on a non-baseline triple (e.g. wasm32 or aarch64)
<!-- AC:END -->
