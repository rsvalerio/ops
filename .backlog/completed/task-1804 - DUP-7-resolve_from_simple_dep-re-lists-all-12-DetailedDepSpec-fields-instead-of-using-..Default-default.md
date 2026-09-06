---
id: TASK-1804
title: >-
  DUP-7: resolve_from_simple_dep re-lists all 12 DetailedDepSpec fields instead
  of using ..Default::default()
status: Done
assignee:
  - TASK-1994
created_date: '2026-08-27 11:26'
updated_date: '2026-08-28 20:18'
labels:
  - code-review-rust
  - duplication
dependencies: []
modified_files:
  - extensions-rust/cargo-toml/src/inheritance.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/inheritance.rs:194-210` (`resolve_from_simple_dep`), against `extensions-rust/cargo-toml/src/types.rs:474-491` (the hand-written `Default for DetailedDepSpec`).

**What**: `DetailedDepSpec` already has a `Default` impl whose values are exactly what `resolve_from_simple_dep` writes for every field it does not compute:

```rust
DetailedDepSpec {
    version: Some(version.to_string()),
    path: None, git: None, branch: None, tag: None, rev: None,
    features: local_features,
    optional: local_optional,
    default_features: local_default_features,
    workspace: None, package: None, target: None,
}
```

Nine of the twelve lines restate `Default`. The same struct is built a second time in `resolve_from_detailed_dep` (`:232-248`) with a different field-by-field spelling, so `DetailedDepSpec` is now constructed exhaustively in three places (including the `Default` impl itself).

**Why it matters**: DUP-7 — this is the failure mode `..Default::default()` exists to prevent. `DetailedDepSpec` is `#[non_exhaustive]`, so adding a field (a new cargo dependency key — `registry`, `public`, `artifact`) is a compile error at all three sites, and the reviewer must decide the right value three times with no single place stating the default. The three copies can drift silently in the one direction that still compiles: a wrong-but-valid value in one constructor. Rewriting as

```rust
DetailedDepSpec {
    version: Some(version.to_string()),
    features: local_features,
    optional: local_optional,
    default_features: local_default_features,
    ..DetailedDepSpec::default()
}
```

leaves only the fields this resolver actually decides, which is also the clearer statement of intent — the doc comment on `resolve_dep_from_workspace` (`:159-172`) already says the local source fields are deliberately discarded, and `..Default::default()` says that in code.

Low severity: no current defect, and the `default_features: true` default is correctly reproduced in both constructors today.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 resolve_from_simple_dep constructs DetailedDepSpec with struct-update syntax over DetailedDepSpec::default(), listing only version, features, optional and default_features
- [x] #2 resolve_from_detailed_dep is reviewed the same way: fields it copies verbatim from the workspace spec stay explicit, fields it deliberately clears fall through to Default where that matches
- [x] #3 Existing resolver tests in src/tests/inheritance.rs pass unchanged, in particular resolve_simple_ws_dep_with_local_optional_and_features and resolve_detailed_ws_dep_propagates_git_fields
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Landed in wave TASK-1994.

AC #1: `resolve_from_simple_dep` now lists only version/features/optional/
default_features over `..DetailedDepSpec::default()`.

AC #2: `resolve_from_detailed_dep` was reviewed and deliberately left exhaustive, with
the rationale recorded on the function. Every field there except `workspace` is copied
from the workspace spec, so the exhaustive literal is the compile-time guard that a
newly added cargo dependency key gets propagated rather than silently defaulted away —
the opposite trade-off from the simple-dep constructor, which decides only four fields.
<!-- SECTION:NOTES:END -->
