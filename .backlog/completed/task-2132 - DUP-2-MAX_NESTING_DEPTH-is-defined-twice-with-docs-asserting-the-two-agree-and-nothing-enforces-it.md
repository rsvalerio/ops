---
id: TASK-2132
title: 'DUP-2: MAX_NESTING_DEPTH is defined twice with docs asserting the two agree, and nothing enforces it'
status: Done
assignee: []
created_date: '2026-09-08 06:55'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-rust
  - duplication
dependencies: []
parent_task_id: 'TASK-2237'
modified_files:
  - extensions/config-checkers/src/json.rs
  - extensions/config-checkers/src/yaml.rs
priority: low
ordinal: 48000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/config-checkers/src/json.rs:32`, `extensions/config-checkers/src/yaml.rs:33`

**What**: the same bound is declared independently in both parser modules:

- `json.rs:32` — `pub const MAX_NESTING_DEPTH: u64 = 128;`, documented as "Matches `serde_json`'s own `RECURSION_LIMIT` so the strict and lenient branches agree on the same input."
- `yaml.rs:33` — `pub const MAX_NESTING_DEPTH: u64 = 128;`, documented as "Matches [`crate::json::MAX_NESTING_DEPTH`] so the two checkers agree on what 'too deep' means."

The yaml constant's doc states a cross-module equality as a fact, but it is a
copied literal, not a derived value, and no test asserts
`json::MAX_NESTING_DEPTH == yaml::MAX_NESTING_DEPTH`. Both are `pub`, so both
are part of the crate's API and a downstream caller can observe the skew.

Each module's tests recompute the expected message from its *own* constant
(`json.rs:220`, `json.rs:243`, `yaml.rs:205`), so raising one of the two
leaves the whole suite green while `check-json` and `check-yaml` disagree
about what "too deep" means.

**Why it matters**: DUP-2 — the duplication is invisible because the two
sites are in different files and the comment does the reassuring. The invariant
the docs advertise ("the two checkers agree") is the one thing the code does
not check. The failure is quiet in the direction that matters: a YAML bound
raised for a legitimate deeply-nested config leaves JSON rejecting the same
shape, and the docs still claim parity.

The two `128`s are also load-bearing security limits (TASK-1809, TASK-1808),
so drift is a correctness change to a guard, not a style nit.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 MAX_NESTING_DEPTH has one definition; the second module re-exports or references it rather than restating the literal
- [ ] #2 If two distinct constants are kept deliberately, a test asserts json::MAX_NESTING_DEPTH == yaml::MAX_NESTING_DEPTH and the docs say why they are separate
- [x] #3 The existing depth tests in json.rs and yaml.rs still pass unchanged
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
AC #1 satisfied via the re-export branch: yaml.rs now does `pub use crate::json::MAX_NESTING_DEPTH` (one definition; the doc comment no longer asserts parity in prose, it states single-sourcing). AC #2 is the "two distinct constants kept deliberately" alternative — not taken, so not applicable. AC #3: all existing depth tests in json.rs and yaml.rs pass unchanged (33/33).
<!-- SECTION:NOTES:END -->
