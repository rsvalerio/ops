---
id: TASK-2739
title: 'ERR-6: workspace-inherited default-features fold disagrees with cargo because ''unspecified'' is a default-true bool'
status: Done
assignee: []
created_date: '2026-11-18 18:38'
updated_date: '2026-11-18 22:30'
labels:
  - code-review-rust
  - error-handling
dependencies: []
parent_task_id: 'TASK-2850'
modified_files:
  - extensions-rust/cargo-toml/src/inheritance.rs
  - extensions-rust/cargo-toml/src/types.rs
priority: medium
ordinal: 1000
dedup_key: 'ERR-6:extensions-rust/cargo-toml/src/inheritance.rs:resolve_from_detailed_dep'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/inheritance.rs:268` (`resolve_from_detailed_dep`), `:226` (`resolve_from_simple_dep`), `:275` (`extract_local_overrides`); `extensions-rust/cargo-toml/src/types.rs:500` (`DetailedDepSpec::default_features`).

**What**: `DetailedDepSpec::default_features` is a `bool` defaulting to `true`, a sentinel that cannot tell "not written" from `default-features = true`. The resolver then folds member and workspace with `ws.default_features && local_default_features`, and the doc comment claims that is cargo's precedence. Cargo's inheritance (`inner_dependency_inherit_with`) is different:

- member `default-features = true`, workspace `default-features = false` -> cargo **enables** default features; the AND fold gives `false`.
- member `default-features = false`, workspace unspecified or `true` (including a bare `dep = "1.0"`) -> cargo **ignores** the member (warning; hard error on edition 2024) and keeps defaults on; the fold, and `resolve_from_simple_dep` which copies the local value, give `false`.

Only the unspecified-member/false-workspace case matches. The `optional` fold (`ws.optional || local_optional`) rests on the same doc claim and should be checked against cargo too.

**Why it matters**: `uses_default_features()` and the provider's JSON report the wrong feature set for inherited deps in both mismatched cases, and the doc comment states the opposite of cargo's documented behaviour, so a reader trusts it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 default-features on DetailedDepSpec distinguishes 'not specified' from an explicit true/false (e.g. Option<bool>), with uses_default_features() still defaulting to true
- [x] #2 Inherited-dep resolution matches cargo: member true + workspace false enables defaults; member false + workspace unspecified/true keeps defaults on
- [x] #3 The resolve_from_detailed_dep doc comment describes cargo's actual precedence, and tests cover all four member/workspace combinations for both simple and detailed workspace deps

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.12 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
